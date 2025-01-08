# Zero Sugar

This is a REPL for the Zero Sugar compiler. It's a tongue in cheek compiler whose goal it is to reduce the syntax surface of the JS language by rewriting certain high levelconstructs ("JSSugar") into atomic building blocks ("JS0").

Note: this is not a serious project. I made it as a joke. In its current form it shouldn't be used for anything serious and is in no way production ready. At this time I'm not planning on making this anything serious either (that would probably end up being more of a rewrite of [Preval](https://github.com/pvdz/preval)) but I'm open to suggestions.

## REPL

There is a simple web REPL at my website: https://pvdz.ee/project/zero-sugar

## Features

Currently converts the following JS/TS syntax away in favor of simpler JS:

- `switch` statements, in favor of `if-else` chains
- `continue` keyword, in favor of labeled `break`
- `finally` blocks, in favor of `try/catch`
- `for` loops, in favor of `while` loops
- `do-while` loops, in favor of `while` loops
- complex variable declarations (with patterns), in favor of step-by-step destructuring

## Usage

This is a Rust project. You have to compile the code to wasm using the build.sh script which requires `wasm-pack` to be installed on your system.

By default it targets the web but you can update the target in the build script to target nodejs instead, after which you can run `example.js` with nodejs (or anything else, I guess).

For the web, you only need to run the `build.sh` script to update the wasm binaries and reload your browser. Due to web platform restrictions you have to run the web repl from a webserver of any kind, a local (or remote) webserver should work fine. It just won't work on `file://`, I believe that's by design so I didn't try to work around it.

## Tests

I use `insta` for snapshot testing. Run `cargo insta test` to run the tests. You can also run plain `cargo test` if you don't have or don't want to use `insta`.

## Stack

This uses [Oxc](https://github.com/oxc-project/oxc) as the parser. I created a mapper for its output. It was my first time working with Oxc and AST's _in Rust_ in general and I'm sure I missed a ton of things that could have been done better.

If you're looking to learn how to use Oxc in a similar way, this project might be a good starting point. I struggled with it initally because I couldn't find an easy samples of how to use the nodes and what not. The lifetimes gave me an especially hard time. The docs were either too overwhelming or missing concrete examples to work with. I'm not even sure it was meant to be consumed this way, but it's certainly possible (:

## Naming

There was a proposal not to long ago to split the syntax spec into two parts; "JS0" and "JSSugar". One would be sort of the building blocks of JS that environments would support and the other would be a spec for compilers such that they can compile to JS0. The idea being that it relieves pressure on engine builders for security and bugs while moving some of that burden to compilers. It also adds a burden to any JS developer to now require a compilation step (for insofar engines don't already do that currently, of course), which would raise the bar of coding stuff in JS.

While I like the idea of JS0, I'm not a fan of raising the bar and making a compilation step mandatory for JS development, even if that's the status quo for production pipelines (typescript, jsx, etc). This compiler compiles already basic JS syntax into even more basic syntax (I'm not sure anyone would propose to move things like `continue` and `finally` to JSSugar but this compiler does it). The big question being where to draw the line.

I liked "zero sugar" because for one it's a perfect pun on JS0/JSSugar and it's also a play on the "diet soft drinks", putting the JS syntax on a "diet".

## More

If you're interested I've been trying to push this idea further with [Preval](https://github.com/pvdz/preval). That's a compiler written in JS that tries to push some boundaries on compiling JS syntax into more basic building blocks while remaining to be valid JS. It's goal was to try and compile away developer abstractions unnecessary for the production runtime. A never ending work in progress, I guess :)

This compiler could apply all the things that older "6 to 5" compilers used to do (compiling es6 features to es5 or even es3). And beyond that it could be much more aggressive in trying to come up with a "MISC JS" ("minimal instruction set JS"), which is what JS0 probably wouldn't even want to target :p Eliminating `continue` and `finally` are good examples of this.

More things you could do:

- Eliminate patterns
- Eliminate variations of syntax
    - Force all statements with sub-statements to be a block (`{}`)
    - Force all while loops to be `while (true)`
    - Force all `if` statements to have an `else`
- Eliminate variable / function hoisting
- Eliminate arrows in favor of functions
- Consolidate the `arguments` name and ban it after compilation
- Eliminate scoping complexities by forcing every variable in the code to be unique
- Force all labels to be unique
- Squash labels that nest directly
- Eliminate TS non-runtime code

That sort of thing. What would be the MISC of JS syntax? How far can you reasonably push it?

## Bugs

Yeah, probably. Sorry-not-sorry.
