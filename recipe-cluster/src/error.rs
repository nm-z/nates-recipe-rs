use core::fmt;

#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum ClusterError {
	InvalidConfiguration(String),
	MissingMember(String),
	DuplicateMember(String),
	StaleMember(String),
	InvalidMember { member: String, message: String },
	MissingNetworkMeasurement(String),
	DuplicateNetworkMeasurement(String),
	InvalidNetworkMeasurement(String),
	IdentityExhausted(&'static str),
	InvalidClusterProfile(String),
	Codec(String),
}

impl fmt::Display for ClusterError {
	fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
		match self {
			Self::InvalidConfiguration(message) => write!(formatter, "invalid cluster configuration: {message}"),
			Self::MissingMember(member) => write!(
				formatter,
				"member {member} did not submit a measured profile"
			),
			Self::DuplicateMember(member) => write!(formatter, "member {member} appears more than once"),
			Self::StaleMember(member) => {
				write!(
					formatter,
					"member {member} submitted a stale or unexpected measured profile"
				)
			}
			Self::InvalidMember { member, message } => {
				write!(
					formatter,
					"member {member} has an invalid measured profile: {message}"
				)
			}
			Self::MissingNetworkMeasurement(pair) => {
				write!(
					formatter,
					"network pair {pair} has no measured probe evidence"
				)
			}
			Self::DuplicateNetworkMeasurement(pair) => {
				write!(
					formatter,
					"network pair {pair} has duplicate measured probe evidence"
				)
			}
			Self::InvalidNetworkMeasurement(message) => {
				write!(formatter, "invalid inter-machine measurement: {message}")
			}
			Self::IdentityExhausted(kind) => write!(formatter, "{kind} identity space is exhausted"),
			Self::InvalidClusterProfile(message) => write!(formatter, "invalid cluster profile: {message}"),
			Self::Codec(message) => write!(formatter, "cluster profile codec: {message}"),
		}
	}
}

impl std::error::Error for ClusterError {}

pub type ClusterResult<T> = Result<T, ClusterError>;
