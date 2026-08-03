use core::fmt;

use crate::NodeId;

/// A one-based source position plus its zero-based UTF-8 byte offset.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SourceLocation { line: usize, column: usize, offset: usize, }

impl SourceLocation {
	#[must_use]
	pub const fn new(line: usize, column: usize, offset: usize) -> Self { Self { line, column, offset, } }

	#[must_use]
	pub const fn line(self) -> usize { self.line }

	#[must_use]
	pub const fn column(self) -> usize { self.column }

	#[must_use]
	pub const fn offset(self) -> usize { self.offset } }

/// Why text supplied through the arena-building API cannot form one node.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum NodeTextErrorKind { Empty, Tab, CarriageReturn, LineFeed, }

/// Failure to extend an in-memory graph.
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum GraphError { InvalidNodeText { kind: NodeTextErrorKind, character: usize, }, UnknownParent(NodeId), }

impl fmt::Display for GraphError {
	fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
		match self { Self::InvalidNodeText { kind, character } => { write!( formatter,
					"invalid node text at character {character}: {kind:?}"
				) }
			Self::UnknownParent(parent) => write!(formatter, "unknown parent node {}", parent.index()),
		} } }

impl std::error::Error for GraphError {}

/// Why a textual document cannot be parsed.
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum ParseErrorKind { EmptyNode, IndentationJump { found: usize, maximum: usize }, BareCarriageReturn, }

/// A syntax failure with an exact source location.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ParseError { kind: ParseErrorKind, location: SourceLocation, }

impl ParseError {
	#[must_use]
	pub const fn new(kind: ParseErrorKind, location: SourceLocation) -> Self { Self { kind, location } }

	#[must_use]
	pub const fn kind(&self) -> &ParseErrorKind { &self.kind }

	#[must_use]
	pub const fn location(&self) -> SourceLocation { self.location } }

impl fmt::Display for ParseError {
	fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
		let message = match self.kind {
			ParseErrorKind::EmptyNode => "empty node".to_owned(),
			ParseErrorKind::IndentationJump { found, maximum } => {
				format!("indentation depth {found} skips an ancestor; maximum depth is {maximum}")
			}
			ParseErrorKind::BareCarriageReturn => "bare carriage return; use LF or CRLF".to_owned(),
		}; write!( formatter,
			"{message} at line {}, column {}",
			self.location.line(), self.location.column() ) } }

impl std::error::Error for ParseError {}
