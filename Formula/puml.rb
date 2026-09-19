class Puml < Formula
  desc "Rust CLI reimplementation of PlantUML"
  homepage "https://github.com/grahambrooks/puml"
  version "2026.5.24"
  license "MIT"

  on_macos do
    on_arm do
      url "https://github.com/grahambrooks/puml/releases/download/v2026.5.24/puml-2026.5.24-aarch64-apple-darwin.tar.gz"
      sha256 "159e5f6886cdad7992b104fdb866f453dc65eda47b2c0b5896a93ba4962029d2"
    end
  end

  on_linux do
    url "https://github.com/grahambrooks/puml/releases/download/v2026.5.24/puml-2026.5.24-x86_64-unknown-linux-gnu.tar.gz"
    sha256 "5636353e71cbc0e49724563f6f13edbf9be5640bd35b669c4c5f23f305f7bcff"
  end

  def install
    bin.install "puml"
  end

  test do
    system "#{bin}/puml", "--version"
  end
end
