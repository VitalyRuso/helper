# Google Cloud deployment

This is the MVP deployment: one Compute Engine VM runs Caddy, web, API, PostgreSQL, and Qdrant through Docker Compose. PostgreSQL and Qdrant use persistent Docker volumes on the VM disk.

## Resources

- Project number: `745947699440`
- Region: `europe-west1`
- Zone: `europe-west1-b`
- VM: `spain-helper`, `e2-standard-2`
- Disk: 30 GB balanced persistent disk
- Static address: `spain-helper-ip`
- Network/subnet: `spain-helper-net` / `spain-helper-eu`
- Public hostname: `spain-helper-<STATIC-IP-WITH-DASHES>.sslip.io`

Only TCP 80 and 443 are public. SSH is allowed only through Google IAP. API, web, PostgreSQL, and Qdrant host bindings use `127.0.0.1`; Caddy reaches them through the private Compose network.

The permanently billable resources are the VM, its disk, and the in-use static external IPv4 address, plus network egress. Docker volumes survive container rebuilds but not deletion of the VM boot disk.

## Prerequisites

1. Install the Google Cloud CLI.
2. Run `gcloud auth login`.
3. Ensure billing is enabled and the account can manage Compute Engine, Service Usage, firewalls, and IAP SSH.
4. Merge and push the tested application to `origin/main`.
5. Start from the repository root with a clean checkout.

The scripts resolve the project ID from the project number and set the active gcloud project, region, and zone. They also select the SDK's bundled Python when available.

## Deploy

```powershell
.\aplicaciones\deploy\gcp\deploy.ps1
```

The script idempotently creates the VPC, subnet, firewall rules, static IP, and VM. On the VM it:

- installs Docker from Docker's signed Debian repository;
- clones or fast-forwards `main` without reset or force;
- creates `/opt/spain-helper/aplicaciones/.env` with mode `600`;
- generates independent database, admin, access, and worker secrets;
- builds the cloud Compose stack;
- requires valid public HTTPS before reporting success.

After HTTPS succeeds, only `LOCAL_WORKER_TOKEN` is copied over IAP into the ignored local file `aplicaciones/.env.worker`. The script restricts that file's Windows ACL to the current user. No secret is printed or passed as a command argument.

To retrieve the generated admin password, open an interactive IAP SSH session and read it directly on the VM:

```powershell
gcloud compute ssh spain-helper --zone=europe-west1-b --tunnel-through-iap
sudo sed -n 's/^ADMIN_PASSWORD=//p' /opt/spain-helper/aplicaciones/.env
```

Do not paste secrets into Git, issue trackers, or ordinary logs.

## Update

After `main` is pushed:

```powershell
.\aplicaciones\deploy\gcp\update.ps1
```

The update refuses tracked VM-side edits, uses a fast-forward pull, records the previous revision outside the checkout, and rebuilds the stack.

## Roll back

```powershell
.\aplicaciones\deploy\gcp\rollback.ps1
```

Rollback switches to the recorded commit and rebuilds containers. It does not reverse SQL migrations or restore deleted data; take disk/database backups before risky schema or content changes.

## Verify

The deploy script validates HTTPS without bypassing certificate checks. Additional checks:

```powershell
$Ip = gcloud compute addresses describe spain-helper-ip --region=europe-west1 --format='value(address)'
$HostName = "spain-helper-$($Ip -replace '\.','-').sslip.io"

Invoke-RestMethod "https://$HostName/health"
Invoke-RestMethod "https://$HostName/api/categories"
Invoke-WebRequest "https://$HostName/"

gcloud compute ssh spain-helper --zone=europe-west1-b --tunnel-through-iap `
  --command="cd /opt/spain-helper/aplicaciones && sudo docker compose -f docker-compose.yml -f docker-compose.cloud.yml ps"
```

Worker endpoints require `LOCAL_WORKER_TOKEN`; admin endpoints require admin authentication. PostgreSQL, Qdrant, ports 3000/8000, and LM Studio port 1234 are not public.

If TLS issuance fails, do not disable certificate validation or send the worker token over HTTP. Confirm that the sslip.io hostname resolves to the reserved IP, firewall ports 80/443 are open, and Caddy is running:

```powershell
Resolve-DnsName $HostName
gcloud compute ssh spain-helper --zone=europe-west1-b --tunnel-through-iap `
  --command="cd /opt/spain-helper/aplicaciones && sudo docker compose -f docker-compose.yml -f docker-compose.cloud.yml logs --tail=200 caddy"
```
