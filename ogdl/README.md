# recipe-ogdl

`recipe-ogdl` implements Recipe's deliberately narrow, OGDL-derived textual
graph profile. It represents an ordered forest in an arena:

- ordinary spaces are literal node text;
- leading tabs select a node's parent by indentation depth;
- later tabs on the same line form a parent-child chain; and
- line endings separate chains.

For example:

```text
system	master	device
	workers	worker 0
		worker 1
```

No quoting is needed for spaces. Empty nodes, bare carriage returns, and
indentation that skips an ancestor are rejected with line and column
information. Both LF and CRLF inputs are accepted. Canonical output uses LF,
writes the first child inline, and writes later siblings on indented lines.

The current representation is an ordered rooted forest, not a general graph.
It intentionally has no shared-reference, anchor, link, cycle, comment, escape,
schema, or binary syntax. Those features must be designed and implemented
explicitly before Recipe can claim to support them.
