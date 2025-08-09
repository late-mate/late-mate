**⚠️ not meant for public consumption (yet?) ⚠️**

# Installation

* you will need the following installed globally:
    * [`direnv`](https://direnv.net/)
    * [`just`](https://github.com/casey/just)
    * [`awscli`](https://formulae.brew.sh/formula/awscli)
* add an AWS profile with `aws configure --profile late-mate` (this will create an AWS profile called `late-mate` in
  your `~/.aws/credentials` file)
* `direnv allow` in the repo will load vendored binaries, local npm modules, and `AWS_PROFILE` into your ENV
  (and unload automatically when you exit the directory)