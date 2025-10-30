<h1 align="center" id="title">Simple Language Loader</h1> <p align="center"> <img src="https://socialify.git.ci/Skidy89/simple-language-loader/image?custom_description=a+simple+language+loader+addon+for+nodejs&custom_language=Rust&description=1&forks=1&issues=1&language=1&name=1&owner=1&pulls=1&stargazers=1&theme=Light" alt="simple-language-loader" width="640" height="320" /></p> <p align="center">


Simple Language Loader is a fast and lightweight Rust + N-API addon for Node.js that provides a simple yet powerful way to load `.lang` files with support for arrays, multiline strings, and comments.
It’s designed for multilingual projects that need high performance and easy js/ts integration

## features
- Full TypeScript support
- Automatic caching for repeated loads
- Simple and readable api
- Cross-platform Napi bindings
- Comment support (#)

## Installation
```bash
npm i @skidy89/ssl.js
```

## Example
```ts
import { loadChdlang, generateTypescriptDefs, clearLangCache, load_lang_dsk } from "@skidy89/ssl.js"

// create map cached langs
const langs = loadChdlang("./languages")
// or if you prefer, without any cached langs
const uncachlangs = load_lang_dsk("./languages")

console.log(langs.en.HELLO)
console.log(langs.es.GOODBYE)

// generate typescript files, will throw an error if the path doesnt exist
generateTypescriptDefs("./languages", "./types/langs.d.ts")

// clean the cached languages
clearLangCache()
```

> [!WARNING]
> this project is still on development, if you encounter an error, let me know or make a pull request 