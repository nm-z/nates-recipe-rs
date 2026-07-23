#[cfg(all(feature = "legacy", feature = "next"))]
compile_error!("`legacy` and `next` are mutually exclusive; use `--no-default-features --features next`");
#[cfg(not(any(feature = "legacy", feature = "next")))]
compile_error!("select exactly one Recipe facade: `legacy` or `next`");

#[cfg(feature = "legacy")]
include!("facade_legacy.rs");

#[cfg(feature = "next")]
include!("facade_next.rs");
