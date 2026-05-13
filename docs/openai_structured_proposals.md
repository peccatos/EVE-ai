# OpenAI Structured Proposals

EVE uses OpenAI only as a proposal layer. EVE remains responsible for
sanitization, schema validation, approval, safe apply, validation, and evidence.

Structured proposal JSON must include:

```json
{
  "summary": "string",
  "files_to_change": ["docs/example.md"],
  "risk_level": "low",
  "patch_ops": [
    {
      "path": "docs/example.md",
      "op": "CreateFile",
      "description": "string",
      "content": "string"
    }
  ]
}
```

Malformed JSON, forbidden paths, path traversal, absolute paths, self-approval,
or shell/apply instructions are refused before approval.
