# Measured profile schema

Schema and codec version 5 retain canonical topology-origin metadata:

- every topology machine has exactly one retained `MachineFingerprint`;
- every RAM device has exactly one retained discovery-domain key;
- every disk device has exactly one retained storage-domain key;
- every GPU-memory device has exactly one retained native GPU key;
- machine fingerprints are ordered by `MachineId`;
- RAM, storage, and GPU origins are independently ordered by `DeviceId`;
- stable machine IDs are unique within a profile;
- each origin kind's keys are unique within its machine.

The codec rejects missing, duplicate, unknown, mis-typed, or non-canonical
origins. The probe engine constructs and validates the complete profile before
returning or caching it.

Version 5 preserves the version 4 peer-link evidence contract:
evidence:

- exact authenticated local and remote machine/profile digests;
- whether the two directions ran simultaneously or serially;
- total bytes, elapsed nanoseconds, and sample count per direction;
- minimum, maximum, mean, and integer variance of per-frame durations; and
- the benchmark protocol schema and exact duration-derived directional rates.

The codec checks that evidence matches the bounded network plan and that each
stored rate is exactly derivable from its byte and elapsed-time evidence.

Cache, topology, and discovery hash domains are versioned with `v5`. Version 4
profile bytes are intentionally incompatible and fail closed. The default
`recipe probe` cache filename includes the schema and therefore moves to a new
`measured-v5-...` file without overwriting a version 4 cache.

Cluster assembly uses the retained stable machine ID for endpoint machine
identity and resolves peer gateway RAM by its retained domain key. Measured
capacity and rate are consistency checks only; they are never used to infer an
identity. Native realization likewise reopens RAM, storage, CUDA, and HSA
devices by these retained keys and fails if the exhaustive current inventory
does not match.
