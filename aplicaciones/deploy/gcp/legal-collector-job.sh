#!/usr/bin/env bash
set -euo pipefail

PROJECT_ID="${PROJECT_ID:-project-4d660d0c-8615-4485-834}"
PROJECT_NUMBER="${PROJECT_NUMBER:-745947699440}"
REGION="${REGION:-europe-west1}"
ZONE="${ZONE:-europe-west1-b}"
VM_NAME="${VM_NAME:-spain-helper}"
PUBLIC_HOST="${PUBLIC_HOST:-spain-helper-34-38-50-7.sslip.io}"
REPOSITORY="${REPOSITORY:-spain-helper}"
JOB_NAME="${JOB_NAME:-legal-collector}"
SECRET_NAME="${SECRET_NAME:-legal-collector-token}"
SERVICE_ACCOUNT_ID="${SERVICE_ACCOUNT_ID:-legal-collector-job}"
SERVICE_ACCOUNT="${SERVICE_ACCOUNT_ID}@${PROJECT_ID}.iam.gserviceaccount.com"
IMAGE="${REGION}-docker.pkg.dev/${PROJECT_ID}/${REPOSITORY}/legal-collector:latest"

gcloud config set project "${PROJECT_ID}"
gcloud services enable \
  aiplatform.googleapis.com \
  artifactregistry.googleapis.com \
  cloudbuild.googleapis.com \
  run.googleapis.com \
  secretmanager.googleapis.com

gcloud artifacts repositories describe "${REPOSITORY}" --location "${REGION}" >/dev/null 2>&1 ||
  gcloud artifacts repositories create "${REPOSITORY}" \
    --repository-format docker \
    --location "${REGION}" \
    --description "Spain Helper bounded collector images"

gcloud iam service-accounts describe "${SERVICE_ACCOUNT}" >/dev/null 2>&1 ||
  gcloud iam service-accounts create "${SERVICE_ACCOUNT_ID}" \
    --display-name "Spain Helper Legal Collector Job"

gcloud projects add-iam-policy-binding "${PROJECT_ID}" \
  --member "serviceAccount:${SERVICE_ACCOUNT}" \
  --role roles/aiplatform.user \
  --quiet >/dev/null

if ! gcloud secrets describe "${SECRET_NAME}" >/dev/null 2>&1; then
  gcloud secrets create "${SECRET_NAME}" --replication-policy automatic
  TOKEN_FILE="$(mktemp)"
  ENV_FILE="$(mktemp)"
  trap 'rm -f "${TOKEN_FILE}" "${ENV_FILE}"' EXIT
  umask 077
  openssl rand -base64 48 > "${TOKEN_FILE}"
  gcloud secrets versions add "${SECRET_NAME}" --data-file "${TOKEN_FILE}" >/dev/null
  printf 'LEGAL_COLLECTOR_TOKEN=%s\n' "$(cat "${TOKEN_FILE}")" > "${ENV_FILE}"
  gcloud compute scp "${ENV_FILE}" "${VM_NAME}:/tmp/legal_collector.env" \
    --zone "${ZONE}" \
    --tunnel-through-iap
  gcloud compute ssh "${VM_NAME}" \
    --zone "${ZONE}" \
    --tunnel-through-iap \
    --command "sudo sed -i '/^LEGAL_COLLECTOR_TOKEN=/d' /opt/spain-helper/aplicaciones/.env && sudo sh -c 'cat /tmp/legal_collector.env >> /opt/spain-helper/aplicaciones/.env' && sudo chmod 600 /opt/spain-helper/aplicaciones/.env && rm -f /tmp/legal_collector.env"
fi

gcloud secrets add-iam-policy-binding "${SECRET_NAME}" \
  --member "serviceAccount:${SERVICE_ACCOUNT}" \
  --role roles/secretmanager.secretAccessor \
  --quiet >/dev/null

gcloud builds submit . \
  --config aplicaciones/deploy/gcp/cloudbuild.legal-collector.yaml \
  --substitutions "_IMAGE=${IMAGE}"

gcloud run jobs deploy "${JOB_NAME}" \
  --region "${REGION}" \
  --image "${IMAGE}" \
  --service-account "${SERVICE_ACCOUNT}" \
  --tasks 1 \
  --parallelism 1 \
  --max-retries 0 \
  --task-timeout 900 \
  --cpu 1 \
  --memory 1Gi \
  --command /usr/local/bin/legal_collector \
  --set-secrets "LEGAL_COLLECTOR_TOKEN=${SECRET_NAME}:latest" \
  --set-env-vars "API_BASE_URL=https://${PUBLIC_HOST},GOOGLE_CLOUD_PROJECT=${PROJECT_ID},VERTEX_LOCATION=${REGION},VERTEX_FAST_MODEL=gemini-2.5-flash,VERTEX_REVIEW_MODEL=gemini-2.5-pro,LEGAL_COLLECTOR_DRY_RUN=true,LEGAL_COLLECTOR_SMOKE_RUN=true,MAX_DOCUMENTS_PER_RUN=5,MAX_MODEL_REQUESTS_PER_RUN=20,MAX_INPUT_TOKENS_PER_RUN=50000,MAX_OUTPUT_TOKENS_PER_RUN=12000,MAX_ESTIMATED_COST_USD=5,MAX_JOB_DURATION_SECONDS=900,MAX_RETRIES_PER_DOCUMENT=0"

echo "Created/updated Cloud Run Job ${JOB_NAME} using ${IMAGE}."
echo "Dry-run command: gcloud run jobs execute ${JOB_NAME} --region ${REGION} --wait"
