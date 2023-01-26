// https://inlang.com/documentation
export async function defineConfig(env) {
  const plugin = await env.$import(
    "https://cdn.jsdelivr.net/gh/samuelstroschein/inlang-plugin-json@1/dist/index.js"
  );

  const pluginConfig = {
    pathPattern: "./i18n/{language}.json",
  };

  return {
    referenceLanguage: "en",
    languages: [
      "de",
      "en",
      "es",
      "fr",
      "nl",
      "pl",
      "pt-BR",
      "ru",
      "tr",
      "zh-cn",
    ],
    readResources: (args) =>
      plugin.readResources({ ...args, ...env, pluginConfig }),
    writeResources: (args) =>
      plugin.writeResources({ ...args, ...env, pluginConfig }),
  };
}
