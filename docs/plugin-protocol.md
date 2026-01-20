# Hamr Plugin Protocol Reference

This document describes the JSON protocol used between plugins and the hamr-rs daemon.

## Transport
Plugins communicate over stdin/stdout (stdio) or via a Unix socket using newline-delimited JSON.

## Requests
Each request is a JSON object with the following common fields:

- `step`: `"initial" | "search" | "action" | "form"`
- `query`: string
- `selected`: optional object containing the selected item
- `action`: optional string identifying an action button
- `formData`: optional object mapping field IDs to values
- `session`: string session identifier

### Example
```json
{"step":"search","query":"calc 1+1","session":"abc"}
```

## Responses
Responses are JSON objects with a `type` field.

### results
```json
{
  "type": "results",
  "results": [{"id":"1","name":"Result","description":"Example"}]
}
```

Fields:
- `id`: string
- `name`: string
- `description`: optional string
- `icon`: optional string
- `iconType`: optional `"system" | "material" | "text"`
- `thumbnail`: optional string
- `badges`, `chips`, `actions`: optional arrays

### execute
```json
{"type":"execute"}
```

### form
```json
{"type":"form","form":{}}
```

### status
```json
{"type":"status","status":{}}
```

Status may include:
- `badges`, `chips`
- `description`
- `fab`: `{ chips, badges, priority, showFab }`
- `ambient`: array

### error
```json
{"type":"error","message":"Something went wrong"}
```

## Notes
- Unknown fields should be ignored for forward compatibility.
- All field names use camelCase.
