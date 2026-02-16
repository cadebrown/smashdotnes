# Infrastructure

This repo uses OpenTofu to manage Cloudflare Pages infrastructure for `smash.cade.io`.

## Scope

* Cloudflare Pages project (`smashdotnes`)
* Custom subdomain (`smash.cade.io`)

## Prerequisites

* OpenTofu installed (`tofu`)
* Cloudflare API token with least-privilege permissions
* Cloudflare Pages connected to your GitHub account

## Token permissions (minimum)

* Account: Cloudflare Pages - Edit
* Zone: Zone - Read
* Zone: DNS - Edit

## Setup (simple)

```sh
cd infra/cloudflare
cp terraform.tfvars.example terraform.tfvars
```

Edit `infra/cloudflare/terraform.tfvars`:

* `account_id = "YOUR_CLOUDFLARE_ACCOUNT_ID"`
* `github_owner = "YOUR_GITHUB_USER_OR_ORG"`

Then:

```sh
export CLOUDFLARE_API_TOKEN="..."
tofu init
tofu plan
tofu apply
```

## File layout

All OpenTofu config for this site lives in one directory:

* `infra/cloudflare/main.tf`
* `infra/cloudflare/build.sh`

## What this does

* Creates/updates the Cloudflare Pages project `smashdotnes`
* Configures Cloudflare Pages build command to compile the Rust/Dioxus site
* Binds `smash.cade.io` to the Pages project
* Creates/updates DNS record for `smash.cade.io`

## What to commit vs ignore

Commit:

* `infra/cloudflare/main.tf`
* `infra/cloudflare/build.sh`
* `infra/cloudflare/terraform.tfvars.example`
* `infra/cloudflare/.terraform.lock.hcl`

Do not commit (already ignored in `infra/.gitignore`):

* `infra/cloudflare/terraform.tfvars`
* `infra/cloudflare/.terraform/`
* `infra/cloudflare/*.tfstate` and backups

## Notes

* No GitHub Actions deployment workflow is required.
* Cloudflare Pages runs `bash infra/cloudflare/build.sh` for deploys.

## References

* Cloudflare Pages build configuration: https://developers.cloudflare.com/pages/configuration/build-configuration/
* Cloudflare Pages build image: https://developers.cloudflare.com/pages/configuration/build-image/
* Cloudflare provider `cloudflare_pages_project`: https://registry.terraform.io/providers/cloudflare/cloudflare/latest/docs/resources/pages_project
