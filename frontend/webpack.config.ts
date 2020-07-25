import * as path from "path";
import * as webpack from "webpack";

const relative = (file: string) => path.resolve(__dirname, file);

const config: webpack.Configuration = {
  context: relative("."),
  entry: {
    nav: {
      import: relative("./src/nav.ts"),
      dependOn: "common",
    },
    search: {
      import: relative("./src/search.ts"),
      dependOn: "common",
    },
    thread: {
      import: relative("./src/thread.ts"),
      dependOn: "common",
    },
    common: relative("./src/common.ts"),
  },

  resolve: {
    extensions: [".ts"],
  },
  output: {
    filename: "[name].js",
    path: relative("../static/js"),
  },
  module: {
    rules: [
      {
        test: /.tsx?$/,
        use: "ts-loader",
        exclude: "/node_modules/",
      },
    ],
  },
  optimization: {
    usedExports: true,
  },
};

export default config;
