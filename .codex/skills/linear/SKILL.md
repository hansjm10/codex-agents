---
name: linear
description: |
  Use Linear GraphQL for raw Linear reads and writes from this repository,
  preferring a `linear_graphql` tool when available and otherwise falling back
  to `curl` with `.env`.
---

# Linear GraphQL

Use this skill when a Codex session in this repository needs to read or update
Linear issues, comments, states, project links, or relations.

## Primary tool

If a `linear_graphql` tool is available in the current session, prefer it.

Tool input shape:

```json
{
  "query": "query or mutation document",
  "variables": {
    "optional": "graphql variables object"
  }
}
```

## Local fallback

If no `linear_graphql` tool is available, use the local `.env` file and call
Linear directly:

```sh
set -euo pipefail
source .env
curl -sS https://api.linear.app/graphql \
  -H "Content-Type: application/json" \
  -H "Authorization: $LINEAR_API_KEY" \
  --data-binary @- <<'EOF'
{"query":"query { viewer { id name } }"}
EOF
```

## Rules

- Send one GraphQL operation per tool call.
- Treat a top-level `errors` array as a failed operation.
- Query only the fields needed for the current action.
- Prefer issue `id` once it is known; use `issues(filter: ...)` only to find
  the initial issue.
- Do not print the Linear API key in logs or comments.

## Common operations

### Resolve issue details

```graphql
query IssueById($id: String!) {
  issue(id: $id) {
    id
    identifier
    title
    url
    description
    state {
      id
      name
      type
    }
    project {
      id
      name
      slugId
    }
    comments {
      nodes {
        id
        body
      }
    }
  }
}
```

### Read team states

```graphql
query IssueTeamStates($id: String!) {
  issue(id: $id) {
    team {
      id
      key
      name
      states {
        nodes {
          id
          name
          type
        }
      }
    }
  }
}
```

### Update issue state

```graphql
mutation UpdateIssueState($id: String!, $stateId: String!) {
  issueUpdate(id: $id, input: { stateId: $stateId }) {
    success
    issue {
      id
      identifier
      state {
        name
      }
    }
  }
}
```

### Create a workpad comment

```graphql
mutation CreateComment($issueId: String!, $body: String!) {
  commentCreate(input: { issueId: $issueId, body: $body }) {
    success
    comment {
      id
      body
    }
  }
}
```

### Update an existing workpad comment

```graphql
mutation UpdateComment($id: String!, $body: String!) {
  commentUpdate(id: $id, input: { body: $body }) {
    success
    comment {
      id
      body
    }
  }
}
```

### Create a related or blocking issue relation

```graphql
mutation CreateRelation($issueId: String!, $relatedIssueId: String!, $type: IssueRelationType!) {
  issueRelationCreate(input: { issueId: $issueId, relatedIssueId: $relatedIssueId, type: $type }) {
    success
  }
}
```

### Create an issue

```graphql
mutation CreateIssue($teamId: String!, $title: String!, $description: String, $stateId: String) {
  issueCreate(input: { teamId: $teamId, title: $title, description: $description, stateId: $stateId }) {
    success
    issue {
      id
      identifier
      title
      url
    }
  }
}
```

## Expected workflow usage

- Use the team and project context already established for this repository's
  work before creating or mutating issues.
- Keep issue descriptions structured: summary, scope, and acceptance criteria.
- Prefer a small number of well-scoped issues over one oversized catch-all.
