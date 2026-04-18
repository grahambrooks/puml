class Puml < Formula
  desc "Rust CLI reimplementation of PlantUML"
  homepage "https://github.com/grahambrooks/puml"
  version "2026.4.18-3"
  license "MIT"

  on_macos do
    on_arm do
      url "https://github.com/grahambrooks/puml/releases/download/v2026.4.18-3/puml-2026.4.18-3-aarch64-apple-darwin.tar.gz"
      sha256 "2c72938911f1ea336f5bc4545a8e8fccd7dac87117d3aee910f8beac9c6e43ef"
    end
  end

  on_linux do
    url "https://github.com/grahambrooks/puml/releases/download/v2026.4.18-3/puml-2026.4.18-3-x86_64-unknown-linux-gnu.tar.gz"
    sha256 "d0c5d1295d6aa6d7d3830c0c5d759df9afb9d200aa3adfb0fc8fd4bb80f09b62"
  end

  def install
    bin.install "puml"
  end

  test do
    system "#{bin}/puml", "--version"
  end
end
