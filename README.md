# Risp 

The code behind [this essay](https://m.stopa.io/risp-lisp-in-rust-90a0dad5b116)


## Nic's extensions

Nic has extended Risp in a number of small ways:


### strings

nicrisp has strings.

The strings don't have any escaping in them right now.

strings are double quoted lists of characters.


### separation between display values and values

strings have a display value like:

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

#### httpget <url>

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
(httpget "https://google.com")
=> (200
    (("content-type" "text/html")
     (transfer-encoding "chunked"))
    BODY
   )
```

I haven't coded the parts for the body yet.

#### num <max> [<start>]

Takes a `max` (an int) and an optional `start` (an int, by default
`0`) and returns a list of numbers between `start` and `max`.

## static compilation

nicrisp is intended to be statically compiled so you can more easily
use it in heterogenous environments, perhaps you don't have the libssl
available on this platform... but nicrisp static would still have a
functioning `httpget`.


_fin_
