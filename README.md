# Simulgine

[![Coverage Status](https://coveralls.io/repos/github/BattyBest/Simulgine/badge.svg?branch=main)](https://coveralls.io/github/BattyBest/Simulgine?branch=main)
[![Rust](https://github.com/BattyBest/Simulgine/actions/workflows/rust.yml/badge.svg)](https://github.com/BattyBest/Simulgine/actions/workflows/rust.yml)

## The "Why?"

Making simulations is annoying in most programming languages. You have to set
up a bunch of boilerplate, and make it fast too, simply so that you can press a
button and have a value increment according to a value. Simulgine aims to make 
this easier by removing the need for all of that.

## The "What?"

Instead of writing out procedural code that details step by step for the
computer how to get from state A of the simulation to state B of the
simulation, for each value, you write out what you need to do for the next
tick.

## Getting Started

Simulgine can be easily compiled with cargo:
```bash
cargo build --release
```

Afterwards, the binary is a REPL run with one argument for a directory of
**.sml** files.

Each .sml file is a collection of **classes**, which contain **fields**. These
fields all come attached with code, the field's **body**, which computes the
next value of the field for the next tick.

Every Simulgine program must have a **ROOT** class that is the parent of all
other classes.

A basic 'Hello, World!' would be this:

```test_projects/hellosim```
```Simulgine
class ROOT {
    string hello "Hello, world!";
}
```

In order to run this, save it in a file (let's say `hello.sml`) and put it in a
directory (let's say `hellosim`). You can then run it as (in hellosim's parent
directory):
```
/path/to/Simulgine hellosim
```

This will open the REPL:
```SimulgineREPL
Simulgine REPL Terminal
>> 
```

At anywhere in the code, the root can be accessed with the keyword `root`.
It is case-insensitive:
```SimulgineREPL
Simulgine REPL Terminal
>> rOoT
[ROOT] ROOT {
    hello: string = ""
}
```

The square brackets denote the type of the printed value, whilst the ':' is the
type of the field.

Currently, the `hello` field is an empty string. This is because that is the
default value for a string - you can create a default value of any type by
executing `instanceof [type]`. (You can go in reverse by going `typeof [value]`.)

However, since we have given the field a body, the body will be executed on
next tick. Tick the system by executing `!tick` in the REPL.

```SimulgineREPL
Simulgine REPL Terminal
>> rOoT
[ROOT] ROOT {
    hello: string = ""
}
>> !tick
>> rOoT
[ROOT] ROOT {
    hello: string = "Hello, world!"
}
```

This can not be a true hello world, becuase Simulgine does not print anything
by itself. It merely runs the "simulation".

You can quit the REPL with `!quit`.

## Fields

### Keywords

Fields can reference their previous value with the keyword `this`. This program
will simply count the number of ticks passed:

```test_projects/simple_counter```
```Simulgine
class ROOT {
    u64 counter this + 1;
}
```

Fields can reference another field in their class with the keyword `parent`.

```test_projects/parent_and_counter```
```Simulgine
class ROOT {
    u64 three 3;
    u64 counter this + parent.three;
}
```

### Initializers

Fields can be initialized with an expression running in a **const-context**.
In a const-context, you cannot access root, any other user-defined class,
`parent`, or `this`. To actually initialize the field, you must, after the first
semicolon, add an equals sign and then the expression. Of course, it must end
with another semicolon.

The following code demonstrates this, setting the counter to start as 2:

```test_projects/initty_field```
```Simulgine
class ROOT {
    u64 counter this + 1; = 2;
}
```

## Braces

You can use curly braces in fields. This may be for purely aesthetics purposes,
or to chain multiple statements together. The last statement will be returned,
and all statements must end with a semicolon.

```test_projects/simple_braces```
```Simulgine
class ROOT {
    string hello {
        "this does nothing.";
        "Hello, world!";
    };
}
```

You can also use them anywhere else:

```test_projects/lots_braces```
```Simulgine
class ROOT {
    string hello {
        {
            "this does nothing.";
            "still doing nothing.";
        };
        {
            {
                {
                    {
                        "indentation generator";
                    };
                };
            };
        };
        "Hello, world!";
    };
}
```

### Access and Volatility modifiers

Fields have three access modifiers:
 - `public`: Can be accessed from any other class.
 - `protected`: Can be accessed from the same class. **Default**.
 - `private`: Cannot be accessed from any other field.

And also three volatility modifiers:
 - `const`: This value never changes except by external code.
 - `level`: For fields that are not instances of a class, same as `const`. For
    fields that are, the object is ticked when the parent is.
 - `volatile`: The value is recomputed from scratch every tick. **Default**.

Access and volatility modifiers come before the type, and in that order.

Const and level fields do not have a body. Where the body would go, goes the
initializer.

```test_projects/const_field```
```Simulgine
class ROOT {
    const u8 three 3;
}
```

## Staging

Simulgine, by default, computes all the values **at once**. This is a flagship
feature, and allows for massive parallelization, as well as eliminating all
race conditions. This means that every field can only see stale values from
other fields.

This is acceptable in most cases, but sometimes syncronization is required. In
this case, a turbofish can be added after a field's name in order to order it.
By default, fields have a stage of **1**. A field will compute only after all
fields in the class with a lower stage than it have finished being computed,
and will see the newer values.

This will take two `!tick`s for `follower` to become `"Hello, world!"`:

```test_projects/nostaging```
```Simulgine
class ROOT {
    string hello "Hello, world!";
    string follower parent.hello;
}
```

This will only take one `!tick`:

```test_projects/yesstaging```
```Simulgine
class ROOT {
    string hello "Hello, world!";
    string follower::<2> parent.hello;
}
```

Be careful with this feature, especially on classes higher up in the hirearchy.
Staging directly interferes with Simulgine's regular functioning of
parallelization.

## If

Simulgine allows for if statements. If statements are worded like so:
```
if [condition] [onTrue] {optional: else [onFalse]}
```

`onTrue` and `onFalse` are both **expressions** and *not* statements. They
do not end with a semicolon unless surrounded by curly braces. The if statement
itself is also an expression: It returns None without an else clause (except in
the REPL) and returns whichever subexpression was computed when it does have an
else clause.

Try this:

```test_projects/ifwithbrace```
```Simulgine
class ROOT {
    u64 counter this + 1;

    string hello::<2> {
      if parent.counter > 3
        "counter greater than three"
      else
        "counter less than or equal to three";
    };
}
```

The curly braces are purely aesthetics here; this works too:

```test_projects/ifwobrace```
```Simulgine
class ROOT {
    u64 counter this + 1;

    string hello::<2>
      if parent.counter > 3
        "counter greater than three"
      else
        "counter less than or equal to three";
}
```

## Variables

Some complex calculations are more convenient with intermediate values. For
these, Simulgine offers variables, which are declared with the following:
```
let [name]: [type] = [initial value];
```

The variable can then be accessed with its name:
```Simulgine
{
    let randomInteger: u64 = 3;

    randomInteger; // returns 3
}
```

Variables cannot be used outside of curly braces, and they belong to their
braces. They can also be aliased.
```Simulgine
class ROOT {
    u64 counter let previous = this; // No no no no!!!
}
```

```test_projects/lotslets```
```Simulgine
class ROOT {
    u64 someRandomValue {
        let useless: u32 = 3;
        let other: u32 = {
            let useless: u16 = 723

            useless; // 723
        };

        other = useless + other; // 726
        let useless: u32 = 8;

        other + useless; // 734
    };
}
```

You can access an outer braces variable from a more inner one, as well.

```test_projects/lotserletser```
```Simulgine
class ROOT {
    u64 someRandomValue {
        let useless: u32 = 3;
        let moreUseless: u16 = 723;
        let other: u32 = {
            let useless: u16 = moreUseless;

            useless;
        };

        other = useless + other;
        let useless: u32 = 8;

        other + useless;
    };
}
```

## Nesting

Classes may contain other classes in their fields. Volatility modifier of level
is most appropriate in most situations. Please note that staging only applies
within classes, not between them.

```test_projects/nesting```
```Simulgine
class Finances {
    double income 5.0;
    double expenses 2.0;
    public double profit::<2> parent.income - parent.expenses;
}

class Company {
    double stockPrice::<2> this + parent.finances.profit / 10.0;

    level Finances finances;
}

class ROOT {
    level Company incorporationsIncorporated;
}
```
