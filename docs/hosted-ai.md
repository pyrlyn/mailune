# Paid hosted AI tier (design)

Status: design only (task A33). The adapter comes after this design is approved. The creator decided that the paid tier comes later (`AGENTS.md`, decisions of 2026-10-08), so nothing here is built yet.

## Goal

Offer people who have no capable device and no API key of their own a hosted model, run by Mailune, that the operator cannot read. The tier sits behind the existing privacy router in `mailune-ai` as step 5 of the chain in `docs/architecture.md`. It does not replace local AI, which stays the default.

## Provider

**Microsoft Azure confidential GPU VMs, `NCCads_H100_v5` series.** In this series, Microsoft's documentation says the Trusted Execution Environment spans the confidential VM on the CPU and the attached GPU, so data, models and computation can be moved to the GPU securely. Each VM has one NVIDIA H100 NVL GPU (94 GB) and 4th-generation AMD EPYC Genoa processors. Source: https://learn.microsoft.com/en-us/azure/virtual-machines/sizes/gpu-accelerated/nccadsh100v5-series (page updated 2026-07-28, checked 2026-10-08). The confidential GPU driver and attestation setup are linked from that page (https://aka.ms/cgpu-onboarding-steps). This design did not verify the details of that setup; read them before the adapter is built.

Why this one:

- The inference server runs inside a CPU and GPU TEE, so the plaintext prompt exists only inside attested memory.
- The VM is a general-purpose confidential VM. Mailune can run its own open-weights model and its own inference server there, so the same model family can be evaluated by A30 cassettes, locally and hosted.

Other confidential-inference services exist. None was checked for this design, so none is named here. Any of them would need the same attestation check below.

## How it fits the router

| Router piece | Change |
| --- | --- |
| `ModelKind` | New `Hosted` kind. The order becomes platform, local, cloud (BYOK), hosted. |
| `PrivacyClass` | No new class. Hosted is a cloud call, so it needs `CloudAllowed` for that feature, set by the person. |
| Encrypted mail | `effective_privacy` already forces `LocalOnly`. Hosted is skipped exactly like BYOK cloud. A test in the adapter task asserts it. |
| Redaction | `redact_for_cloud` runs before the request is built, as in A6 `build_request`. |
| Ledger | Every call writes a `FlowRecord` with provider `hosted`, bytes and message ids (A7). |
| Agent tools | Unchanged. The hosted model proposes typed calls only. The A9/A28 policy and the app's confirmation stay on the device. |

## Trust protocol

1. The client fetches the enclave's attestation evidence: the CPU TEE report, the GPU attestation, and a hash of the inference image.
2. The client checks the evidence against values pinned in the app release: the expected image hash and the vendors' root certificates. On any mismatch the router treats the hosted model as absent and fails closed. It does not fall back to a weaker path.
3. The client opens a TLS session whose key is bound to the attested report, so the operator's load balancer only ever sees ciphertext.
4. Requests carry no account id and no mailbox address. Billing uses an anonymous token bought separately, so the inference VM never learns who is asking.
5. The inference image keeps no logs of prompt or completion text. The image is reproducible, and its hash is published with each release.

## Out of scope for the first adapter

- Training on user data. There is none, and the image cannot keep it.
- Sending encrypted mail. This is never allowed, on any tier.
- Hosted embeddings for search. Retrieval stays on the device.

## Next tasks (to propose in `ideas.md` or `roadmap.md` once this design is approved)

- Adapter crate for the hosted provider: attestation verification and the request shape (reuse the A6 OpenAI-compatible shape for the inference server).
- Billing token service that is separate from inference.
- Reproducible build of the inference image.
