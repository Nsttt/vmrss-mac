class Vmrss < Formula
  desc "Inspect resident memory usage by process tree on macOS"
  homepage "https://github.com/Nsttt/vmrss-mac"
  version "0.1.1"
  license "MIT"

  depends_on :macos

  on_macos do
    on_arm do
      url "https://github.com/Nsttt/vmrss-mac/releases/download/v0.1.1/vmrss-v0.1.1-macos-arm64.tar.gz"
      sha256 "a51ea9bb48b97341f2de4de3d8038be0d5a73320b991a8f58f067781784ccedb"
    end

    on_intel do
      url "https://github.com/Nsttt/vmrss-mac/releases/download/v0.1.1/vmrss-v0.1.1-macos-x86_64.tar.gz"
      sha256 "fc622e636c32c2508b5c275ba0f33c9980e16787cdafb0593dcee46b98a8650e"
    end
  end

  def install
    bin.install "vmrss"
  end

  test do
    assert_match "total:", shell_output("#{bin}/vmrss #{Process.pid}")
  end
end
