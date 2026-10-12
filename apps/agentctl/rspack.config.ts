import { readFileSync } from "node:fs";
import { rspack } from "@rspack/core";

const pkg = JSON.parse(
  readFileSync(new URL("./package.json", import.meta.url), "utf8"),
);

export default {
  mode: "production",
  target: "node",
  entry: "./src/cli.ts",
  output: { filename: "cli.cjs", clean: true },
  resolve: { extensions: [".ts", ".js"] },
  module: {
    rules: [
      {
        test: /\.ts$/,
        loader: "builtin:swc-loader",
        options: {
          jsc: { parser: { syntax: "typescript" }, target: "es2023" },
        },
        type: "javascript/auto",
      },
    ],
  },
  optimization: { minimize: false },
  plugins: [
    new rspack.BannerPlugin({
      banner: "#!/usr/bin/env node",
      raw: true,
      entryOnly: true,
    }),
    new rspack.DefinePlugin({
      __AGENTCTL_VERSION__: JSON.stringify(pkg.version),
    }),
  ],
};
