// Development-only oracle generator; Rust tests consume JSON without Node.
// Run with Node 26: node tests/fixtures/generate.mjs /path/to/already-tested-js
import { execFileSync } from "node:child_process";
import { mkdirSync, rmSync, writeFileSync } from "node:fs";
import { resolve } from "node:path";
import { pathToFileURL } from "node:url";

const { verifyClaim, createClaim } = await import(
  pathToFileURL(resolve(process.argv[2], "src/claims.js"))
);
const cwd = resolve("tests/fixtures/.generation");
for (const name of ["TMPDIR", "TMP", "TEMP"]) process.env[name] = cwd;
const git = (args, input) => execFileSync("git", args, {
  cwd, input, env: { ...process.env, GIT_NO_REPLACE_OBJECTS: "1" },
}).toString().trim();

for (const unix of [false, true]) {
  rmSync(cwd, { recursive: true, force: true });
  mkdirSync(cwd, { recursive: true });
  const workflow = unix ? '.github/workflows/test "é".yaml' : ".github/workflows/test-é.yaml";
  const check = "unit:linux/x";
  const files = [
    { path: workflow, mode: "100644", bytes: [...Buffer.from("name: Test\non: push\njobs: {}\n")] },
    { path: "src/𐀀", mode: "100644", bytes: [0, 10, 255, 13, 128] },
    { path: "src/\ue000", mode: "100755", bytes: [...Buffer.from('quotes"\\\t\u2028\u2029\n')] },
    { path: unix ? 'src/line\n"tab\t\\file' : "src/line\u2028separator", mode: "100644", bytes: [...Buffer.from("bc")] },
    { path: "src/a", mode: "100644", bytes: [...Buffer.from("d")] },
    { path: "src/empty", mode: "100644", bytes: [] },
    { path: "README.md", mode: "100644", bytes: [...Buffer.from("# Fixture\n")] },
  ];
  try {
    git(["init", "-q", "-b", "master"]);
    git(["config", "user.name", "Fixture"]);
    git(["config", "user.email", "fixture@example.invalid"]);
    git(["config", "commit.gpgsign", "false"]);
    git(["config", "core.autocrlf", "false"]);
    git(["config", "core.hooksPath", resolve(cwd, "no-hooks")]);
    function indexFile(file) {
      const oid = git(["hash-object", "-w", "--stdin"], Buffer.from(file.bytes));
      git(["update-index", "-z", "--index-info"], Buffer.from(`${file.mode} ${oid}\t${file.path}\0`));
    }
    files.forEach(indexFile);
    const source = git(["commit-tree", git(["write-tree"]), "-m", "Source"]);
    git(["update-ref", "HEAD", source]);
    // Only the Unix development host checks out these unusual paths.
    git(["checkout-index", "-a", "-f"]);
    const cases = [];
    for (const [name, patterns] of [
      ["all-files", []],
      ["pattern-policy", ["\ufeff src/** \ufeff", "src/\ue000", "src/𐀀", "src/**"]],
      ["ecmascript-whitespace", ["\u0085", "src/**"]],
    ]) {
      cases.push({
        name, patterns, claim: createClaim({ cwd, workflow, check, files: patterns }),
      });
    }
    const replacement = {
      path: workflow, mode: "100644", bytes: [...Buffer.from("name: Executing workflow\n")],
    };
    indexFile(replacement);
    const executing = git(["commit-tree", git(["write-tree"]), "-p", source, "-m", "Workflow"]);
    git(["update-ref", "HEAD", executing]);
    git(["checkout-index", "-a", "-f"]);
    const override = createClaim({ cwd, workflow, check, files: ["src/**"] });
    if (!verifyClaim({
      cwd, workflow, check, files: ["src/**"], ref: source, workflowRef: executing, body: override,
    })) {
      throw new Error("Oracle workflow override failed.");
    }
    cases.push({
      name: "executing-workflow", patterns: ["src/**"], replacement, claim: override,
    });
    writeFileSync(`tests/fixtures/v1${unix ? "-unix" : ""}.json`, JSON.stringify({
      provenance: "Generated with the unchanged already-tested-js src/claims.js using Node 26. No Node runtime is used by Rust tests.",
      workflow, check, files, cases,
    }, null, 2) + "\n");
  } finally {
    rmSync(cwd, { recursive: true, force: true });
  }
}
