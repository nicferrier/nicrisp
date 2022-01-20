# Risp 

Risp is a toy lisp in Rust that was written by Stepan Parunashvili as
a demonstration of the Norvig "make a lisp" method.

Stepan wrote a good essay about the process [here](https://m.stopa.io/risp-lisp-in-rust-90a0dad5b116).

Nic looked at this and is wondering whether it worked as a vehicle for
some things about lanaguages that he's been thinking about.

Can we make a statically compiled scripting language which has
"batteries included"?


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
