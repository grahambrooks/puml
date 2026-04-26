class Puml < Formula
  desc "Rust CLI reimplementation of PlantUML"
  homepage "https://github.com/grahambrooks/puml"
  version "2026.4.26"
  license "MIT"

  on_macos do
    on_arm do
      url "https://github.com/grahambrooks/puml/releases/download/v2026.4.26/puml-2026.4.26-aarch64-apple-darwin.tar.gz"
      sha256 "5941951fc98567d53a4c02e7c80704015b857cb8c2d83beb0ab664bcc638d3ba"
    end
  end

  on_linux do
    url "https://github.com/grahambrooks/puml/releases/download/v2026.4.26/puml-2026.4.26-x86_64-unknown-linux-gnu.tar.gz"
    sha256 "bef899e071df0bb3c490d29f4517c97610e0dd228f07160da1edfa808842ea41"
  end

  def install
    bin.install "puml"
  end

  test do
    system "#{bin}/puml", "--version"
  end
end
