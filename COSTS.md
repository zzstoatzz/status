# COSTS.md

> **Mandate.** This file tracks the infrastructure costs attributed to **status** and
> what we're doing to bring them down. It is the running record of what this project
> spends, why, and a changelog of changes that moved the number. If the spend here is
> unjustified, that's a signal to clean up — not to ignore it.

## current cost — fetch it live, never hardcode

Costs drift, so this file deliberately does **not** hardcode a dollar figure. Get the
current monthly cost for this repo from the daily snapshot (collected by
`my-prefect-server`, surfaced at https://hub.waow.tech):

```bash
curl -s https://hub.waow.tech/api/costs.json | jq '{
  as_of: .generatedAt,
  this_repo_monthly_usd: (
    [ .lineItems[] | select(.service as $s | ["zzstoatzz-quickslice-status"] | index($s)) ]
    | (map(.amount) | add // 0) / 100
  ),
  lines: [ .lineItems[] | select(.service as $s | ["zzstoatzz-quickslice-status"] | index($s))
           | {service, provider, usd: (.amount/100), estimated} ]
}'
```

Or open the costs panel at https://hub.waow.tech and group **by project**.

Services attributed to this repo: `zzstoatzz-quickslice-status`. If that list is
wrong, fix the mapping in `my-prefect-server`
(`packages/mps/src/mps/costs/projects.py`) rather than editing numbers here.

## how we might bring this down
- biggest line is usually `zzstoatzz-quickslice-status` — check its utilization and right-size before anything else.
- Fly figures are **estimates** from machine inventory — reconcile against the Fly dashboard, and enable auto-stop on bursty/idle machines.

## changelog
- **2026-06-17** — initial cost notice; 1 service(s) attributed here. Run the command above for the live figure.
