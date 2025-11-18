# Dropkick

A tool to help you drop code files into a project from your library of project patterns.  This tool currently works with the Ruby-based [Bundlegem](https://github.com/thenotary/bundlegem) project templating framework.  As you define templates, it can sometimes be helpful to pull in just a few files from your templates as you work, such as a CI/ CD `.gitlab-ci.yaml` file for instance.

As time permits, I may figure out an elegant way to port that over into the Rust world, but in the


## Installation

First install Bundlegem using Ruby and create a symlink to make the `bundlegem` reference be accesible as `dropkick`... I'm in the process of renaming bundlegem, btw...

```
gem install bundlegem
bundlegem --install-public-templates
ln -s ~/.bundlegem ~/.dropkick
```

Now that we have the generic templates available, you'll be able to write your own templates for programming styles you prefer and place them in `~/.dropkick/templates`.  Defining your own project templates is highly recommended!

Now install this repository's binary, `dropkick`.

```
cargo install dropkick

# I recommend aliasing dropkick as dk, you only get so many keystrokes per day!
alias dk="dropkick"
```

## Tutorial

Now we can simulate making a rust project using the vanilla configurations provided by cargo (ideally you might start from your own personally customized template).

```
# CD somewhere disposable where you have execute permissions
cd /tmp
cargo new some-example
git add .
git commit -m "inits repo"

# Now use dropkick's TUI to drop in a Dockerfile from template-rust-wasm-http
dk
```

After selecting the appropriate file with the space bar and hitting `e` to extract the file from the template, you should now see the Dockerfile in your working directory, ready for use.

# Road Map

## Features (COMPLETE)

- `dropkick` - Opens an interactive TUI folder view of all local templates.  You can pull in files or kicklets from this interface.
- Support interpolation of ERB templating

## Features (WIP)

- Get it to prompt you to specify your project name if the .dropkickrc file is missing
- `dropkick --checkout template/file_name` - Checks out a file from a template and drops it into the local folder
- `dropkick --kicklet kicklet_name` - Checks out all files from a "kicklet" into the working tree.  A kicklet is defined in a template's `bundlegem.yaml` file or something? It should allow all related aspects of a pattern to be injected into existing files without ruining things.
- Do some kind of port for bundlegem to be installable as a single cargo command

