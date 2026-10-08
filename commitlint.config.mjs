// Local commitlint rule. pyrlyn/ci at 27290ae30105592f683c236fc76dcc2ee4348bdf
// ships license-check.yml and cla.yml and does not ship a commitlint workflow,
// so this file is the trailer rule and .github/workflows/commitlint.yml is what
// rejects the commit in CI. Node is not pinned in this repository; the workflow
// applies the same check in bash rather than invoking this config.
//
// A message fails when any line is a Co-Authored-By trailer. The human is the
// only author.

export default {
  plugins: [
    {
      rules: {
        "no-agent-attribution": ({ raw = "" }) => {
          const line = raw.match(/^co-authored-by:.*$/im)?.[0];
          return [!line, `the human is the only author; remove: ${line}`];
        },
      },
    },
  ],
  rules: { "no-agent-attribution": [2, "always"] },
};
