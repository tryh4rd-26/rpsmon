class Rps < Formula
  desc "Advanced system resource monitor with beautiful TUI graphs"
  homepage "https://github.com/yourusername/rps"
  url "https://github.com/yourusername/rps/archive/refs/tags/v0.1.0.tar.gz"
  sha256 "PLACEHOLDER_SHA256"
  license "MIT"

  depends_on "rust" => :build

  def install
    system "cargo", "install", "--locked", "--root", prefix, "--path", "."
  end

  test do
    # Just verify the binary exists
    assert_predicate bin/"rps", :exist?
  end
end
