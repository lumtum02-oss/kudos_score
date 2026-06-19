# kudos_score

## Project Title
kudos_score

## Project Description
Communities — open-source projects, DAOs, hackathon crews, study groups — have
no portable, on-chain way to record peer recognition. Likes, reactions, and
"shout-outs" today live in centralized apps that can be edited, deleted, or
silently biased. kudos_score is a small Soroban smart contract that turns
peer recognition into a public, append-only reputation score. Any signed
address can award a kudos of a chosen category (`code`, `docs`, `community`,
`design`, ...) to another participant; the recipient accumulates a weighted
score that any other contract, frontend, or governance process can read to
gate participation or rank contributors. There is no XLM transfer and no
off-chain dependency — just a signed on-chain attestation of
"I, address X, give kudos of category Y to address Z with weight W".

## Project Vision
We believe reputation earned from peers is the most honest form of credibility
on the internet, and that it should belong to the recipient, not to a
platform. The long-term goal of kudos_score is to be a primitive that other
Stellar dApps can compose: DAOs can use the score to gate voting power on
small but important decisions, hackathons can use it to surface genuinely
helpful mentors, and grant committees can use it to identify contributors
with a sustained track record. Each kudos is a receipt, every receipt is
public, and the score grows only with work that other people noticed.

## Key Features
- **Peer-to-peer kudos** — any authenticated address can give one kudos of a
  named category to any other address. Self-kudos are blocked at the
  contract level.
- **Weighted categories** — the admin configures a `Map<Symbol, u32>` of
  category weights (e.g. `code: 5`, `docs: 3`, `community: 2`) at
  initialization and can update them later via `set_category_weight`.
  Categories not configured default to weight 1. New weights apply to
  future kudos; historical entries keep their original weight.
- **Append-only history with revoke** — every kudos is stored as a
  `KudosEntry { from, category, weight, timestamp, message_hash, active }`.
  The original giver can revoke a kudos (e.g. issued in error), which
  subtracts its weight from the recipient's total and per-category score
  and flips the entry to inactive. The entry is never deleted, so the
  full history of a recipient is auditable.
- **On-chain events** — `kudos/give`, `kudos/revoke`, and `kudos/setwt`
  events are published on every state change, so indexers and frontends
  can build live reputation feeds without re-scanning storage.
- **Governance-gating ready** — `get_score`, `get_score_by_category`,
  `list_kudos`, and `get_kudos` are pure read calls, so any other
  contract can call them at zero cost to itself to gate proposals,
  voting, or airdrops.

## Contract

- **Network:** Stellar Testnet (Public)
- **Scope:** identity dApp — see `contracts/kudos_score/src/lib.rs` for the full kudos_score business logic.
- **Functions exposed:** see `Key Features` above and the `pub fn` list in `lib.rs`.
- **Contract ID:** `CBEFZ2WBXTWTTK4LT4SS2RQHUEQ6ICH3J67WXVROZSAD4ZSINKYSDEX2`
- **Explorer template:** `https://stellar.expert/explorer/testnet/tx/014c322cbe991e63ea4ac4b05944be501f1b43a17c299c1d5033e3705fc79c40`
- **Screenshot of deployed contract on Stellar Expert:**
  `_(Screenshot of the contract page on Stellar Expert will appear here after deploy.)_`


## Future Scope
- **TTL extension helper** — wrap `give_kudos` and the read endpoints with
  `extend_ttl` on the recipient's persistent entries so a quiet recipient
  does not silently lose their reputation history.
- **Time-decay score** — add a `get_score_at(env, recipient, timestamp)`
  view that weights historical kudos by age, so reputation reflects "what
  you have done lately" rather than a lifetime total.
- **Anti-sybil guard** — add an optional `require_min_score` argument on
  `give_kudos` so low-reputation addresses cannot farm a single
  high-weight recipient, and let admins cap the number of kudos one giver
  can issue per day.
- **Cross-contract composability** — expose a `require_score(recipient,
  threshold)` view that other Soroban contracts can call from their own
  auth logic to gate votes, claims, or access to private storage.
- **Off-chain message rendering** — document the expected `message_hash`
  format (e.g. `sha256(ipfs_cid || memo)`) so frontends can fetch the
  full reason text from IPFS / Arweave and render it next to each entry.
- **Sub-category hierarchies** — allow `category: "code.rust"` to inherit
  weight from `code`, so admins only configure parents.
- **Frontend dashboard** — a small web UI built with `@stellar/freighter-api`
  that connects a wallet, lists the recipient's top kudos, and lets
  authenticated givers issue a new kudos in a few clicks.

## Profile

- **Name:** <!-- Fill github name -->
- **Project:** `kudos_score` (identity)
- **Built with:** Soroban SDK 25, Rust, Stellar Testnet
