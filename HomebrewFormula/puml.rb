class Puml < Formula
  desc "Rust CLI reimplementation of PlantUML"
  homepage "https://github.com/grahambrooks/puml"
  version "2026.4.18-4"
  license "MIT"

  on_macos do
    on_arm do
      url "https://github.com/grahambrooks/puml/releases/download/v2026.4.18-4/puml-2026.4.18-4-aarch64-apple-darwin.tar.gz"
      sha256 "ed240155b21bcd4bbc81dad314bfaa5b7565fad834dd61c3022c654286c65c64"
    end
  end

  on_linux do
    url "https://github.com/grahambrooks/puml/releases/download/v2026.4.18-4/puml-2026.4.18-4-x86_64-unknown-linux-gnu.tar.gz"
    sha256 "648bf152732f7ec00f7ab681f463342fcfb550752f89e60f02175937f984673b"
  end

  def install
    bin.install "puml"
  end

  test do
    system "#{bin}/puml", "--version"
  end
end
