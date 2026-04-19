class Puml < Formula
  desc "Rust CLI reimplementation of PlantUML"
  homepage "https://github.com/grahambrooks/puml"
  version "2026.4.19-2"
  license "MIT"

  on_macos do
    on_arm do
      url "https://github.com/grahambrooks/puml/releases/download/v2026.4.19-2/puml-2026.4.19-2-aarch64-apple-darwin.tar.gz"
      sha256 "a379cb29ad8f023506dfd348d6906156fdc9f7527ae2a237a4ebf8f6a8162673"
    end
  end

  on_linux do
    url "https://github.com/grahambrooks/puml/releases/download/v2026.4.19-2/puml-2026.4.19-2-x86_64-unknown-linux-gnu.tar.gz"
    sha256 "2669ae47b6929dd3334561b6351428d31bc98d8c72a100f5dff136e20b3f7403"
  end

  def install
    bin.install "puml"
  end

  test do
    system "#{bin}/puml", "--version"
  end
end
