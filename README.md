# nicrisp 

nicrisp is a toy+1 based on the lisp in Rust that was written by
Stepan Parunashvili as a demonstration of the Norvig "make a lisp"
method.

Stepan wrote a good essay about the process [here](https://m.stopa.io/risp-lisp-in-rust-90a0dad5b116).

Nic looked at this and is wondering whether it worked as a vehicle for
some things about lanaguages that he's been thinking about.

Can we make a statically compiled scripting language which has
"batteries included"?

## Risp core language

Let's just have a simple summary here:

```
(def a 10)
=> a
(def b 20.0)
=> b
(+ a b)
=> 30
(def b 20.1)
=> b
(+ a b)
=> 30.1
(> a b)
=> false
(<= a b)
=> true
(def f (fn (x) (+ 1 x)))
=> f
(f 10)
=> 11
(if (< a b) 7 9)
=> 7
(def a 45.7)
=> a
(if (< a b) 7 9)
=> 9
```

That's all Risp is. Very simple.

## Nic's extensions

Nic has extended Risp in a number of small ways:

### comments

nicrisp has two styles of comment: Lisp comments and Shell comments.

Lisp comments are as usual for Lisps, a single `;` starts a comment
that carries till the end of the current line. Conventionally Lisp
programmers use double `;` sometimes.

Shell comments are used in Unix shells, like bash and begin with the
hash or pound symbol, depending on your locale: `#` and continue till
the end of the line.

These are all legal comments and equivalent in nicrisp:

```
; this is a comment
;; this is a comment
# this is a comment
######### this is a comment
```

There are no multiline comments in nicrisp.

A more complete code example, perhaps?

```
(def a "hello") ;; comment
```

### strings

nicrisp has strings.

The strings don't have any escaping in them right now.

strings are double quoted lists of characters.

### json structures

nicrisp has support for json objects.

There is currently no parser support for json values so the only way
they could currently get in your program would be `httpget`.

Literal support in the parser will follow.


### separation between display values and values

nicrisp strings have a display value and a reader form like:

```
"hello"
```

but the value (obviously and naturally) does not include the quotes.

There are no other types that have a different printable form right
now.

## loops - kind of

A basic looping map like builtin exists:

```
(repeat (fn (x) (+ x 7)) (num 3))
=> (7,8,9)
```

The first argument must be a lambda and the second a list.

`repeat` iterates over each item of the list and executes the lambda
against it.

The return values are collected and returned as a list.

Another example:

```
(def l (num 4))
(def f (fn (x) (+ 4 x)))
(repeat f l)
=> (4,5,6,7)
```

### self evaluating symbols

Symbols beginning with `:` evaluate to themselves, eg: the value of
`:symbol` is `:symbol`.


### additional functions

#### httpget \<url\>

Takes a url argument and returns a list:

```
(
 status-code [integer]
 header-list [list of lists of strings]
 body
 )
```

For example:

```
(httpget "https://jsonplaceholder.typicode.com/posts/1")
=> (200,
    (("content-type" "application/json")
     (transfer-encoding "chunked")),
    { "data": "object",
      "key": "value",
      "number-value": 10 }
   )
```

NB: this is not an exact representation of the JSON at the specified
url.

#### num \<max\> \[\<start\>\]

Takes a `max` (an int) and an optional `start` (an int, by default
`0`) and returns a list of numbers between `start` and `max`.

## static compilation

nicrisp is intended to be statically compiled so you can more easily
use it in heterogenous environments, perhaps you don't have the libssl
available on this platform... but nicrisp static would still have a
functioning `httpget`.

## bad stuff

The nicrisp parser still sucks.

It does not handle multiline forms _at all_.

So you _cannot_ write:

```
(def
  a
  "hello")
```

in nicrisp yet.


_fin_
