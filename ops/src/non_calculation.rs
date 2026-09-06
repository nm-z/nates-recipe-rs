use crate::OperationFamily;

/// Compatibility entries whose behavior is orchestration or metadata rather
/// than payload arithmetic. These entries never authorize a CPU calculation
/// fallback.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NonCalculationRecipe {
	FacadeDeclaration,
	TextTokenization,
	ModelContainerParsing,
	ChatTemplateRendering,
	RunShutdown,
	EliminatedVendorWorkspaceBinding,
}

impl NonCalculationRecipe {
	pub(crate) fn for_entry(symbol: &str, source: &str) -> Option<Self> {
		let recipe = match symbol {
			"Data" | "Model" | "Train" | "Infer" => Self::FacadeDeclaration,
			"encode" => Self::TextTokenization,
			"parse_safetensors" => Self::ModelContainerParsing,
			"render_chat" | "render_template" => Self::ChatTemplateRendering,
			"gpu_shutdown" => Self::RunShutdown,
			"gpu_blas_workspace" => Self::EliminatedVendorWorkspaceBinding,
			_ if source.contains("safetensors") => Self::ModelContainerParsing,
			_ => return None,
		};
		Some(recipe)
	}

	#[must_use]
	pub const fn definition(self) -> &'static str {
		match self {
			Self::FacadeDeclaration => "typed facade declaration that records data, model, training, or inference configuration without payload arithmetic",
			Self::TextTokenization => "deterministic raw-text to int32 token-ID metadata transformation performed before payload admission",
			Self::ModelContainerParsing => "validated container parsing that constructs tensor metadata and opaque byte ranges before admission",
			Self::ChatTemplateRendering => "deterministic raw-text template rendering performed before tokenization and payload admission",
			Self::RunShutdown => "exit lifecycle action replaced by the typed one-free-per-device executor transition",
			Self::EliminatedVendorWorkspaceBinding => "prohibited vendor-library workspace setter eliminated; owned primitive scratch is fixed during prepare",
		}
	}

	#[must_use]
	pub const fn operation_family(self) -> OperationFamily {
		match self {
			Self::FacadeDeclaration => OperationFamily::Facade,
			Self::TextTokenization | Self::ChatTemplateRendering => OperationFamily::Encoding,
			Self::ModelContainerParsing => OperationFamily::Parsing,
			Self::RunShutdown => OperationFamily::Lifecycle,
			Self::EliminatedVendorWorkspaceBinding => OperationFamily::Workspace,
		}
	}
}
