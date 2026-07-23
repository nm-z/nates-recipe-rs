use crate::model::normalize_display_path;
use crate::policy::classify_dependency;
use crate::{AuditError, Finding, FindingCategory};
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet, VecDeque};

/// One Cargo package in an injected dependency graph.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DependencyPackage {
	pub id: String,
	pub name: String,
	pub manifest_path: String,
}

impl DependencyPackage {
	#[must_use]
	pub fn new(id: impl Into<String>, name: impl Into<String>, manifest_path: impl Into<String>) -> Self {
		Self {
			id: id.into(),
			name: name.into(),
			manifest_path: normalize_display_path(&manifest_path.into()),
		}
	}
}

/// One directed `package -> dependency` edge.
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct DependencyEdge {
	pub package_id: String,
	pub dependency_id: String,
}

impl DependencyEdge {
	#[must_use]
	pub fn new(package_id: impl Into<String>, dependency_id: impl Into<String>) -> Self {
		Self {
			package_id: package_id.into(),
			dependency_id: dependency_id.into(),
		}
	}
}

/// Explicit roots and the complete graph needed to audit their closure.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DependencyGraph {
	pub packages: Vec<DependencyPackage>,
	pub edges: Vec<DependencyEdge>,
	pub root_package_ids: Vec<String>,
}

impl DependencyGraph {
	#[must_use]
	pub fn new(packages: Vec<DependencyPackage>, edges: Vec<DependencyEdge>, root_package_ids: Vec<String>) -> Self {
		Self {
			packages,
			edges,
			root_package_ids,
		}
	}

	/// Parse Cargo's `--format-version 1` JSON for exact caller-selected roots.
	///
	/// # Errors
	///
	/// Returns [`AuditError::InvalidCargoMetadata`] for malformed, partial, or
	/// internally inconsistent metadata.
	pub fn from_cargo_metadata_json(json: &str, root_package_ids: &[String]) -> Result<Self, AuditError> {
		if root_package_ids.is_empty() {
			return Err(AuditError::InvalidCargoMetadata(
				"at least one exact root package id is required".into(),
			));
		}
		let document: Value =
			serde_json::from_str(json).map_err(|error| AuditError::InvalidCargoMetadata(error.to_string()))?;
		let object = document
			.as_object()
			.ok_or_else(|| AuditError::InvalidCargoMetadata("top-level value must be an object".into()))?;
		if object.get("version").and_then(Value::as_u64) != Some(1) {
			return Err(AuditError::InvalidCargoMetadata(
				"metadata version must be exactly 1".into(),
			));
		}

		let package_values = object
			.get("packages")
			.and_then(Value::as_array)
			.ok_or_else(|| AuditError::InvalidCargoMetadata("packages must be an array".into()))?;
		let mut packages = Vec::with_capacity(package_values.len());
		for value in package_values {
			let package = value
				.as_object()
				.ok_or_else(|| AuditError::InvalidCargoMetadata("package must be an object".into()))?;
			packages.push(DependencyPackage::new(
				required_string(package.get("id"), "package.id")?,
				required_string(package.get("name"), "package.name")?,
				required_string(package.get("manifest_path"), "package.manifest_path")?,
			));
		}

		let resolve = object
			.get("resolve")
			.and_then(Value::as_object)
			.ok_or_else(|| {
				AuditError::InvalidCargoMetadata(
					"resolve graph is required; metadata cannot use --no-deps".into(),
				)
			})?;
		let nodes = resolve
			.get("nodes")
			.and_then(Value::as_array)
			.ok_or_else(|| AuditError::InvalidCargoMetadata("resolve.nodes must be an array".into()))?;
		let mut edges = BTreeSet::new();
		let mut node_ids = BTreeSet::new();
		for value in nodes {
			let node = value
				.as_object()
				.ok_or_else(|| AuditError::InvalidCargoMetadata("resolve node must be an object".into()))?;
			let id = required_string(node.get("id"), "resolve.nodes[].id")?.to_owned();
			if !node_ids.insert(id.clone()) {
				return Err(AuditError::InvalidCargoMetadata(format!(
					"duplicate resolve node {id:?}"
				)));
			}

			let dependencies = node
				.get("dependencies")
				.and_then(Value::as_array)
				.ok_or_else(|| {
					AuditError::InvalidCargoMetadata("resolve.nodes[].dependencies must be an array".into())
				})?;
			for dependency in dependencies {
				edges.insert(DependencyEdge::new(
					id.clone(),
					required_string(Some(dependency), "dependency package id")?,
				));
			}
		}

		let graph = Self::new(
			packages,
			edges.into_iter().collect(),
			root_package_ids.to_vec(),
		);
		graph.validate()?;
		Ok(graph)
	}

	pub(crate) fn audit(&self) -> Result<Vec<Finding>, AuditError> {
		let packages = self.validate()?;
		let mut adjacency: BTreeMap<&str, Vec<&str>> = BTreeMap::new();
		for edge in &self.edges {
			adjacency
				.entry(edge.package_id.as_str())
				.or_default()
				.push(edge.dependency_id.as_str());
		}
		for dependencies in adjacency.values_mut() {
			dependencies.sort_unstable();
			dependencies.dedup();
		}

		let mut queue = self
			.root_package_ids
			.iter()
			.map(String::as_str)
			.collect::<VecDeque<_>>();
		let mut visited = BTreeSet::new();
		while let Some(id) = queue.pop_front() {
			if !visited.insert(id) {
				continue;
			}
			if let Some(dependencies) = adjacency.get(id) {
				queue.extend(dependencies.iter().copied());
			}
		}

		let mut findings = visited
			.into_iter()
			.filter_map(|id| packages.get(id))
			.filter(|package| classify_dependency(&package.name).is_prohibited())
			.map(|package| {
				Finding::blocking(
					FindingCategory::Dependency,
					package.manifest_path.clone(),
					0,
					package.name.clone(),
				)
			})
			.collect::<Vec<_>>();
		findings.sort();
		findings.dedup();
		Ok(findings)
	}

	fn validate(&self) -> Result<BTreeMap<&str, &DependencyPackage>, AuditError> {
		if self.root_package_ids.is_empty() {
			return Err(AuditError::InvalidDependencyGraph(
				"at least one root package id is required".into(),
			));
		}

		let mut packages = BTreeMap::new();
		for package in &self.packages {
			if package.id.is_empty() || package.name.is_empty() || package.manifest_path.is_empty() {
				return Err(AuditError::InvalidDependencyGraph(
					"package id, name, and manifest path must be nonempty".into(),
				));
			}
			if packages.insert(package.id.as_str(), package).is_some() {
				return Err(AuditError::InvalidDependencyGraph(format!(
					"duplicate package id {:?}",
					package.id
				)));
			}
		}
		for root in &self.root_package_ids {
			if !packages.contains_key(root.as_str()) {
				return Err(AuditError::InvalidDependencyGraph(format!(
					"missing root package {root:?}"
				)));
			}
		}

		let mut edges = BTreeSet::new();
		for edge in &self.edges {
			if !packages.contains_key(edge.package_id.as_str()) {
				return Err(AuditError::InvalidDependencyGraph(format!(
					"edge source is absent: {:?}",
					edge.package_id
				)));
			}
			if !packages.contains_key(edge.dependency_id.as_str()) {
				return Err(AuditError::InvalidDependencyGraph(format!(
					"edge target is absent: {:?}",
					edge.dependency_id
				)));
			}
			if !edges.insert((edge.package_id.as_str(), edge.dependency_id.as_str())) {
				return Err(AuditError::InvalidDependencyGraph(format!(
					"duplicate edge {:?} -> {:?}",
					edge.package_id, edge.dependency_id
				)));
			}
		}
		Ok(packages)
	}
}

fn required_string<'a>(value: Option<&'a Value>, field: &str) -> Result<&'a str, AuditError> {
	value.and_then(Value::as_str)
		.filter(|value| !value.is_empty())
		.ok_or_else(|| AuditError::InvalidCargoMetadata(format!("{field} must be a nonempty string")))
}

#[cfg(test)]
mod tests {
	use super::*;

	fn metadata() -> String {
		serde_json::json!({
		    "version": 1,
		    "packages": [
			  {"id": "root 0.1", "name": "next-root", "manifest_path": "/scope/Cargo.toml"},
			  {"id": "clean 1.0", "name": "architecture", "manifest_path": "/registry/architecture/Cargo.toml"},
			  {"id": "bad 1.0", "name": "rocblas-sys", "manifest_path": "/registry/rocblas/Cargo.toml"},
			  {"id": "unused 1.0", "name": "hip-sys", "manifest_path": "/registry/hip/Cargo.toml"}
		    ],
		    "resolve": {
			  "nodes": [
				{"id": "root 0.1", "dependencies": ["clean 1.0", "bad 1.0"]},
				{"id": "clean 1.0", "dependencies": []},
				{"id": "bad 1.0", "dependencies": []},
				{"id": "unused 1.0", "dependencies": []}
			  ]
		    }
		})
		.to_string()
	}

	#[test]
	fn cargo_metadata_audits_only_explicit_reachable_closure() {
		let graph = DependencyGraph::from_cargo_metadata_json(&metadata(), &["root 0.1".into()]).unwrap();
		let findings = graph.audit().unwrap();
		assert_eq!(findings.len(), 1);
		assert_eq!(findings[0].symbol, "rocblas-sys");
	}

	#[test]
	fn missing_or_partial_graph_fails_closed() {
		let graph = DependencyGraph::new(
			vec![DependencyPackage::new("root", "root", "/root/Cargo.toml")],
			vec![DependencyEdge::new("root", "absent")],
			vec!["root".into()],
		);
		assert!(graph.audit().is_err());
		assert!(DependencyGraph::from_cargo_metadata_json("{}", &["root".into()]).is_err());
	}
}
