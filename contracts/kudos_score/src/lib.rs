#![no_std]

use soroban_sdk::{
    contract, contractimpl, contracttype, symbol_short, Address, BytesN, Env, Map, Symbol,
};

/// Storage keys for the kudos reputation contract.
#[contracttype]
#[derive(Clone)]
pub enum DataKey {
    /// Contract admin address. The only address allowed to mutate category weights.
    Admin,
    /// `Map<Symbol, u32>` of category -> weight configured by the admin.
    CatWeights,
    /// Total accumulated score for a recipient across all categories.
    Score(Address),
    /// Score in a single category for a recipient.
    CatScore(Address, Symbol),
    /// A single kudos entry at the recipient's `index`.
    Kudos(Address, u32),
    /// How many kudos entries a recipient has received (active or revoked).
    KudosCount(Address),
}

/// A single peer-to-peer kudos entry. Stored under
/// `DataKey::Kudos(recipient, index)` and kept forever, even after revoke,
/// so the full history of a recipient's reputation is auditable on-chain.
#[contracttype]
pub struct KudosEntry {
    /// The address that issued the kudos.
    pub from: Address,
    /// The category of the kudos (e.g. `code`, `docs`, `community`, `design`).
    pub category: Symbol,
    /// The score weight applied to the recipient at issuance time. The admin
    /// can change category weights later; historical entries are not rewritten.
    pub weight: u32,
    /// Stellar ledger timestamp when the kudos was given.
    pub timestamp: u64,
    /// 32-byte hash of an off-chain message (e.g. an IPFS CID) describing
    /// the contribution. The contract never sees the plaintext.
    pub message_hash: BytesN<32>,
    /// Whether the kudos still counts towards the recipient's score. Flipped
    /// to `false` by `revoke_kudos`; the entry is never deleted.
    pub active: bool,
}

/// kudos_score — community reputation built from peer recognition.
#[contract]
pub struct KudosScore;

#[contractimpl]
impl KudosScore {
    /// Initialize the contract. Must be called exactly once before any other
    /// function. `admin` is the only address allowed to mutate category
    /// weights; `weights` is the initial `Map<Symbol, u32>` from category
    /// name to the score weight that should be applied when a peer gives
    /// a kudos of that category. Categories not present in the map default
    /// to weight 1.
    pub fn initialize(env: Env, admin: Address, weights: Map<Symbol, u32>) {
        if env.storage().instance().has(&DataKey::Admin) {
            panic!("Already initialized");
        }
        admin.require_auth();
        env.storage().instance().set(&DataKey::Admin, &admin);
        env.storage().instance().set(&DataKey::CatWeights, &weights);
    }

    /// Peer-to-peer kudos: the caller `giver` authorizes this call and awards
    /// one kudos of `category` to `recipient`. The contract looks up the
    /// weight for that category (default 1) and adds it to the recipient's
    /// total and per-category score. `message_hash` is a 32-byte hash of an
    /// off-chain message (e.g. an IPFS CID) describing the contribution.
    /// Returns the recipient's new total kudos score.
    pub fn give_kudos(
        env: Env,
        giver: Address,
        recipient: Address,
        category: Symbol,
        message_hash: BytesN<32>,
    ) -> u32 {
        giver.require_auth();
        if giver == recipient {
            panic!("Cannot give kudos to yourself");
        }

        let weights: Map<Symbol, u32> = env
            .storage()
            .instance()
            .get(&DataKey::CatWeights)
            .unwrap_or(Map::new(&env));
        let weight = weights.get(category.clone()).unwrap_or(1);

        let count: u32 = env
            .storage()
            .persistent()
            .get(&DataKey::KudosCount(recipient.clone()))
            .unwrap_or(0);

        let entry = KudosEntry {
            from: giver.clone(),
            category: category.clone(),
            weight,
            timestamp: env.ledger().timestamp(),
            message_hash,
            active: true,
        };
        env.storage()
            .persistent()
            .set(&DataKey::Kudos(recipient.clone(), count), &entry);

        env.storage()
            .persistent()
            .set(&DataKey::KudosCount(recipient.clone()), &(count + 1u32));

        let total: u32 = env
            .storage()
            .persistent()
            .get(&DataKey::Score(recipient.clone()))
            .unwrap_or(0);
        let new_total = total + weight;
        env.storage()
            .persistent()
            .set(&DataKey::Score(recipient.clone()), &new_total);

        let cat_total: u32 = env
            .storage()
            .persistent()
            .get(&DataKey::CatScore(recipient.clone(), category.clone()))
            .unwrap_or(0);
        env.storage()
            .persistent()
            .set(
                &DataKey::CatScore(recipient.clone(), category.clone()),
                &(cat_total + weight),
            );

        env.events().publish(
            (
                symbol_short!("kudos"),
                symbol_short!("give"),
                recipient.clone(),
            ),
            (giver.clone(), category, weight),
        );

        new_total
    }

    /// The original `giver` revokes a previously awarded kudos stored at the
    /// recipient's `index`. The contract verifies the kudos belongs to the
    /// caller and was still active, then subtracts the original weight from
    /// the recipient's total and per-category score and marks the entry as
    /// inactive. The entry itself is kept for history. `reason` is included
    /// in the emitted on-chain event for transparency. Returns the
    /// recipient's new total kudos score.
    pub fn revoke_kudos(
        env: Env,
        giver: Address,
        recipient: Address,
        index: u32,
        reason: Symbol,
    ) -> u32 {
        giver.require_auth();
        let key = DataKey::Kudos(recipient.clone(), index);
        let mut entry: KudosEntry = env
            .storage()
            .persistent()
            .get(&key)
            .expect("Kudos not found");
        if entry.from != giver {
            panic!("Only the original giver can revoke this kudos");
        }
        if !entry.active {
            panic!("Kudos already revoked");
        }
        entry.active = false;
        env.storage().persistent().set(&key, &entry);

        let total: u32 = env
            .storage()
            .persistent()
            .get(&DataKey::Score(recipient.clone()))
            .unwrap_or(0);
        let new_total = total.saturating_sub(entry.weight);
        env.storage()
            .persistent()
            .set(&DataKey::Score(recipient.clone()), &new_total);

        let cat_key = DataKey::CatScore(recipient.clone(), entry.category.clone());
        let cat_total: u32 = env
            .storage()
            .persistent()
            .get(&cat_key)
            .unwrap_or(0);
        let new_cat = cat_total.saturating_sub(entry.weight);
        env.storage().persistent().set(&cat_key, &new_cat);

        env.events().publish(
            (
                symbol_short!("kudos"),
                symbol_short!("revoke"),
                recipient.clone(),
            ),
            (giver.clone(), index, reason),
        );

        new_total
    }

    /// View: total accumulated kudos score for `recipient` across all categories.
    pub fn get_score(env: Env, recipient: Address) -> u32 {
        env.storage()
            .persistent()
            .get(&DataKey::Score(recipient))
            .unwrap_or(0)
    }

    /// View: kudos score for `recipient` in a single `category`.
    pub fn get_score_by_category(env: Env, recipient: Address, category: Symbol) -> u32 {
        env.storage()
            .persistent()
            .get(&DataKey::CatScore(recipient, category))
            .unwrap_or(0)
    }

    /// View: number of kudos entries recorded for `recipient` (active or revoked).
    /// Useful for paginating through `get_kudos`.
    pub fn list_kudos(env: Env, recipient: Address) -> u32 {
        env.storage()
            .persistent()
            .get(&DataKey::KudosCount(recipient))
            .unwrap_or(0)
    }

    /// View: returns the full `KudosEntry` stored at `recipient`'s `index`.
    pub fn get_kudos(env: Env, recipient: Address, index: u32) -> KudosEntry {
        env.storage()
            .persistent()
            .get(&DataKey::Kudos(recipient, index))
            .expect("Kudos not found")
    }

    /// Admin sets (or updates) the weight for a `category`. Used to tune the
    /// reputation system — for example, `code_review` might weigh more than
    /// `encouragement`. The admin must authorize the call. Future kudos of
    /// that category use the new weight; historical kudos keep their
    /// original weight stored in the entry.
    pub fn set_category_weight(
        env: Env,
        admin: Address,
        category: Symbol,
        weight: u32,
    ) {
        admin.require_auth();
        let stored: Address = env
            .storage()
            .instance()
            .get(&DataKey::Admin)
            .expect("Contract not initialized");
        if stored != admin {
            panic!("Caller is not admin");
        }

        let mut weights: Map<Symbol, u32> = env
            .storage()
            .instance()
            .get(&DataKey::CatWeights)
            .unwrap_or(Map::new(&env));
        weights.set(category.clone(), weight);
        env.storage()
            .instance()
            .set(&DataKey::CatWeights, &weights);

        env.events().publish(
            (symbol_short!("kudos"), symbol_short!("setwt"), admin.clone()),
            (category, weight),
        );
    }

    /// View: returns the weight currently configured for `category`. Defaults
    /// to 1 if the category has not been configured by the admin.
    pub fn get_category_weight(env: Env, category: Symbol) -> u32 {
        let weights: Map<Symbol, u32> = env
            .storage()
            .instance()
            .get(&DataKey::CatWeights)
            .unwrap_or(Map::new(&env));
        weights.get(category).unwrap_or(1)
    }

    /// View: returns the current admin address.
    pub fn get_admin(env: Env) -> Address {
        env.storage()
            .instance()
            .get(&DataKey::Admin)
            .expect("Contract not initialized")
    }
}
