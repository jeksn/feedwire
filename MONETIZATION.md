# Monetization Plan

## Model

**Free with soft limits + one-time Pro unlock. No subscription.**

Inspired by [Tuna for Mac](https://tunaformac.com/pro) ($49 one-time, 3 Mac license, clear free/pro feature table).

- App is free to download and genuinely useful within the free tier
- A single one-time payment unlocks Pro permanently
- License works on a small number of Macs (e.g. 2-3)
- Price TBD — likely in the $15–25 range given FeedWire is a more focused utility than Tuna

## Distribution

- **Direct sales first** (no App Store cut, faster iteration)
- **Checkout: Paddle** (not Stripe) — Paddle acts as merchant of record, handles global VAT/sales tax automatically, no need to manage tax compliance per country
- Mac App Store later, once the product is stable and the $99/year Apple Developer membership makes sense

## Free vs Pro split (to be finalised)

The free tier should be generous enough to be genuinely useful and spread by word of mouth, with Pro unlocking power-user features.

Ideas (not final):
- **Free:** up to ~10 feeds, basic sorting
- **Pro:** unlimited feeds, folders, filter rules, OPML import/export, future features

## Implementation notes (when ready to build)

- Feed limit enforced on the Rust side in `add_feed` command (check count before insert)
- Pro status stored as a validated license key (Paddle webhook sets a flag in `app_settings` table)
- License key entry UI in Settings pane
- Paddle provides a key recovery flow out of the box
