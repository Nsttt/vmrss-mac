---
summary: Homebrew tap publishing workflow and validation steps.
read_when:
  - Updating release packaging or Homebrew formula behavior.
  - Publishing or debugging the Homebrew tap.
---

# Homebrew Tap

The formula lives in `Formula/vmrss.rb` and installs the prebuilt release
archive for the current Mac architecture.

The public tap is `Nsttt/homebrew-taps`. Users install it as:

```sh
brew tap nsttt/taps
brew install vmrss
```

## Publishing The Tap

Create the tap repository if it does not exist:

```sh
gh repo create Nsttt/homebrew-taps --public --description "Homebrew tap for Nestor's CLI tools"
git clone https://github.com/Nsttt/homebrew-taps.git /tmp/homebrew-taps
mkdir -p /tmp/homebrew-taps/Formula
cp Formula/vmrss.rb /tmp/homebrew-taps/Formula/vmrss.rb
cd /tmp/homebrew-taps
git add Formula/vmrss.rb
git commit -m "feat: add vmrss formula"
git push origin main
```

## Release CI

Publishing a GitHub release runs `.github/workflows/release.yml`.

The workflow:

- builds ARM64 and x86_64 macOS release archives
- uploads the archives and SHA-256 files to the GitHub release
- renders a matching `Formula/vmrss.rb`
- commits the formula to `Nsttt/homebrew-taps`

Cross-repository pushes require a repository secret named
`HOMEBREW_TAP_TOKEN`. Use a fine-grained GitHub token with contents read/write
access to `Nsttt/homebrew-taps`.

## Local Validation

Before publishing manually, validate through a local tap:

```sh
brew tap-new Nsttt/taps
cp Formula/vmrss.rb "$(brew --repo Nsttt/taps)/Formula/vmrss.rb"
brew audit --strict --online Nsttt/taps/vmrss
brew install Nsttt/taps/vmrss
brew test Nsttt/taps/vmrss
```

This repository declares the MIT license. Keep `Formula/vmrss.rb` and the CI
formula renderer on `license "MIT"`.
