#!/usr/bin/env -S deno run --allow-read --allow-run=bash,git,cargo --allow-env --allow-sys

import * as zx from 'npm:zx'
import { z, ZodSchema } from 'https://deno.land/x/zod@v3.23.8/mod.ts'
import { assertEquals } from 'https://jsr.io/@std/assert/1.0.0/equals.ts'
import { assert } from 'https://jsr.io/@std/assert/1.0.0/assert.ts'

const CargoTomlSchema = z.object({
  package: z.object({
    name: z.string().min(1),
    description: z.string().min(1),
    repository: z.string().url().min(1),
    metadata: z.object({
      details: z.object({
        title: z.string().min(1),
        tagline: z.string(),
        summary: z.string(),
      }),
    }),
  }),
})

type CargoToml = z.infer<typeof CargoTomlSchema>

const CargoMetadataSchema = z.object({
  packages: z.array(z.object({
    name: z.string(),
    targets: z.array(z.object({
      name: z.string(),
    })),
  })),
})

type CargoMetadata = z.infer<typeof CargoMetadataSchema>

const RepoSchema = z.object({
  url: z.string().url(),
})

type Repo = z.infer<typeof RepoSchema>

const $ = zx.$({ cwd: import.meta.dirname })
const parse = <T>(schema: ZodSchema<T>, input: zx.ProcessOutput) => schema.parse(JSON.parse(input.stdout))

const theCargoToml: CargoToml = parse(CargoTomlSchema, await $`yj -t < Cargo.toml`)
const { package: { name, metadata: { details: { title } } } } = theCargoToml
const theCargoMetadata: CargoMetadata = parse(CargoMetadataSchema, await $`cargo metadata --format-version 1`)
const thePackageMetadata = theCargoMetadata.packages.find((p) => p.name == name)
assert(thePackageMetadata, 'Could not find package metadata')
const target = thePackageMetadata.targets[0]
assert(target, 'Could not find package first target')
const doc = await $`cargo doc2readme --template README.jl --target-name ${target.name} --out -`
const repo: Repo = parse(RepoSchema, await $`gh repo view --json url`)

assertEquals(repo.url, theCargoToml.package.repository)

const autogenerated = `
<!-- DO NOT EDIT -->
<!-- This file is automatically generated by README.ts. -->
<!-- Edit README.ts if you want to make changes. -->
`.trim()

console.info(`
${autogenerated}

# ${title}

[![Build](${repo.url}/actions/workflows/ci.yml/badge.svg)](${repo.url})
[![Documentation](https://docs.rs/${name}/badge.svg)](https://docs.rs/${name})

${doc.stdout}

## Installation

\`\`\`shell
cargo add ${name} tracing_error
\`\`\`

**Important:** add the \`tracing_error\` crate too.

## Gratitude

Like the project? [Say thanks!](${repo.url}/discussions/new?category=gratitude) ❤️

## License

[Apache License 2.0](LICENSE-APACHE) or [MIT License](LICENSE-MIT) at your option.

Unless you explicitly state otherwise, any contribution intentionally submitted for inclusion in this crate by you, as defined in the Apache-2.0 license, shall be dual licensed as above, without any additional terms or conditions.
`.trim())
