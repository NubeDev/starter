-- Tags SCOPE T5 (reconciled): advisory dictionary of known tag keys.
-- The `kind` CHECK enumerates the four canonical kinds — `num` does
-- NOT appear bare, only as `num_discriminant` (the integer-as-string
-- pattern). See `starter_tags::TagKind`.

CREATE TABLE IF NOT EXISTS tag_definitions (
    key         TEXT PRIMARY KEY,
    kind        TEXT NOT NULL,
    description TEXT,
    enum_values JSONB,
    ref_kind    TEXT,
    source      TEXT NOT NULL,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT tag_definitions_kind_valid CHECK (
        kind IN ('bool', 'str', 'ref', 'num_discriminant')
    )
);
