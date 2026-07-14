# COSTS.md

> **Mandate.** This file tracks the infrastructure costs attributed to **status** and
> what we're doing to bring them down. It is the running record of what this project
> spends, why, and a changelog of changes that moved the number. If the spend here is
> unjustified, that's a signal to clean up — not to ignore it.

## current cost — fetch it live, never hardcode

Costs drift, so this file deliberately does **not** hardcode a dollar figure. Get the
current monthly cost for this repo from the public daily snapshot collected by
`my-prefect-server` (also surfaced at https://hub.waow.tech):

```bash
curl -sG https://pds.zzstoatzz.io/xrpc/com.atproto.repo.listRecords \
  --data-urlencode 'repo=zzstoatzz.io' \
  --data-urlencode 'collection=io.zzstoatzz.cost.snapshot' \
  --data-urlencode 'limit=1' | jq '(.records[0].value) as $snapshot | {
  as_of: $snapshot.generatedAt,
  this_repo_monthly_usd: (
    [ $snapshot.lineItems[] | select(.service == "zzstoatzz-quickslice-status") ]
    | (map(.amount) | add // 0) / 100
  ),
  lines: [ $snapshot.lineItems[] | select(.service == "zzstoatzz-quickslice-status")
           | {service, provider, usd: (.amount/100), estimated} ]
}'
```

Expected baseline for the current inventory is **$5.85/month plus bandwidth**:
$5.70 for one continuously running `shared-cpu-1x` Machine with 1GB RAM in
`ewr`, plus $0.15 for the 1GB persistent volume. The shared IPv4/IPv6 addresses
are free. `min_machines_running = 1` means auto-stop does not reduce that compute
floor.

Or open the costs panel at https://hub.waow.tech and group **by project**.

Service attributed to this repo: `zzstoatzz-quickslice-status`, grouped under the
`status` project. If that attribution is
wrong, fix the mapping in `my-prefect-server`
(`packages/mps/src/mps/costs/projects.py`) rather than editing numbers here.

## how we might bring this down
- biggest line is usually `zzstoatzz-quickslice-status` — check its utilization and right-size before anything else.
- Fly figures are **estimates** from machine and volume inventory; bandwidth is not included. Reconcile against the Fly dashboard.
- `min_machines_running = 1` keeps the service warm. If cold starts become acceptable, setting it to `0` is the main remaining cost lever.

## changelog
- **2026-07-14** — corrected Fly pricing to avoid charging twice for RAM included in the CPU preset; added the 1GB volume; attributed the app to the `status` project instead of `misc`. Expected baseline: **$5.85/mo plus bandwidth**.
- **2026-06-17** — initial cost notice; 1 service(s) attributed here. Run the command above for the live figure.
