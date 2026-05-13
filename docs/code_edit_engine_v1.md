# Code Edit Engine v1

Allowed operations:

```text
CreateFile
AppendFile
ReplaceFileIfExists
ReplaceExactText
```

Limits:

```text
max files per proposal: 5
max patch ops per proposal: 10
max content per op: 20 KB
max total proposal content: 80 KB
```

`ReplaceExactText` requires exactly one match. Zero matches return
`exact_text_not_found`; multiple matches return `ambiguous_exact_text_match`.
