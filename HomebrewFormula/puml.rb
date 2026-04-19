class Puml < Formula
  desc "Rust CLI reimplementation of PlantUML"
  homepage "https://github.com/grahambrooks/puml"
  version "2026.4.19"
  license "MIT"

  on_macos do
    on_arm do
      url "https://github.com/grahambrooks/puml/releases/download/v2026.4.19/puml-2026.4.19-aarch64-apple-darwin.tar.gz"
      sha256 "25f8e8f9491884e15fcf160a721afbf00bd4838b7ef077a3e6e174749b83b336"
    end
  end

  on_linux do
    url "https://github.com/grahambrooks/puml/releases/download/v2026.4.19/puml-2026.4.19-x86_64-unknown-linux-gnu.tar.gz"
    sha256 "5c1b922218f43aee9a5cb4f134a3e9b447adbe4a8540e12df58d5e396d4335f2"
  end

  def install
    bin.install "puml"
  end

  test do
    system "#{bin}/puml", "--version"
  end
end
