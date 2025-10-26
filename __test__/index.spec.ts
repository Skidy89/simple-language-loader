import test from 'ava'
import { loadLangs, generateTypescriptDefs } from '../index'
import { Langs } from './languages'
test('load languages', (t) => {
    generateTypescriptDefs("__test__/", "__test__/languages.d.ts")
    const e = loadLangs("__test__/") as Langs
    console.log(e)
    t.is(e.e.hello, 'hello world')
})
test("generate typescript definitions", (t) => {
    generateTypescriptDefs("__test__/", "__test__/languages.d.ts")
    t.pass()
})
