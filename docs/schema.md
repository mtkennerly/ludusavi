These are the schemas produced by the `schema` command.

## `api` command input
```yaml
---
$schema: "http://json-schema.org/draft-07/schema#"
title: Input
description: "The full input to the `api` command."
type: object
required:
  - requests
properties:
  config:
    description: Override configuration.
    default:
      backupDir: ~
    allOf:
      - $ref: "#/definitions/ConfigOverride"
  requests:
    description: The order of the requests here will match the order of responses in the output.
    type: array
    items:
      $ref: "#/definitions/Request"
definitions:
  ConfigOverride:
    description: Overridden configuration.
    type: object
    properties:
      backupDir:
        anyOf:
          - $ref: "#/definitions/FilePath"
          - type: "null"
  FilePath:
    type: string
  FindTitle:
    description: "Find game titles\n\nPrecedence: Steam ID -> GOG ID -> Lutris ID -> exact names -> normalized names. Once a match is found for one of these options, Ludusavi will stop looking and return that match.\n\nDepending on the options chosen, there may be multiple matches, but the default is a single match.\n\nAliases will be resolved to the target title."
    type: object
    properties:
      backup:
        description: Ensure the game is recognized in a backup context.
        default: false
        type: boolean
      disabled:
        description: Select games that are disabled.
        default: false
        type: boolean
      gogId:
        description: Look up game by a GOG ID.
        default: ~
        type:
          - integer
          - "null"
        format: uint64
        minimum: 0.0
      lutrisId:
        description: Look up game by a Lutris slug.
        default: ~
        type:
          - string
          - "null"
      names:
        description: "Look up game by an exact title. With multiple values, they will be checked in the order given."
        default: []
        type: array
        items:
          type: string
      normalized:
        description: "Look up game by an approximation of the title. Ignores capitalization, \"edition\" suffixes, year suffixes, and some special symbols. This may find multiple games for a single input."
        default: false
        type: boolean
      partial:
        description: Select games that have some saves disabled.
        default: false
        type: boolean
      restore:
        description: Ensure the game is recognized in a restore context.
        default: false
        type: boolean
      steamId:
        description: Look up game by a Steam ID.
        default: ~
        type:
          - integer
          - "null"
        format: uint32
        minimum: 0.0
  Request:
    description: An individual request.
    oneOf:
      - type: object
        required:
          - findTitle
        properties:
          findTitle:
            $ref: "#/definitions/FindTitle"
        additionalProperties: false

```

## `api` command output
```yaml
---
$schema: "http://json-schema.org/draft-07/schema#"
title: Output
description: "The full output of the `api` command."
anyOf:
  - type: object
    required:
      - responses
    properties:
      responses:
        description: "Responses to each request, in the same order as the request input."
        type: array
        items:
          $ref: "#/definitions/Response"
  - type: object
    required:
      - error
    properties:
      error:
        description: A top-level error not tied to any particular request.
        allOf:
          - $ref: "#/definitions/Error"
definitions:
  Error:
    type: object
    properties:
      message:
        description: Human-readable error message.
        default: ""
        type: string
  FindTitle:
    type: object
    properties:
      titles:
        description: Any matching titles found.
        default: []
        type: array
        items:
          type: string
        uniqueItems: true
  Response:
    description: A response to an individual request.
    oneOf:
      - type: object
        required:
          - error
        properties:
          error:
            $ref: "#/definitions/Error"
        additionalProperties: false
      - type: object
        required:
          - findTitle
        properties:
          findTitle:
            $ref: "#/definitions/FindTitle"
        additionalProperties: false

```
