class Mdo < Formula
  desc "Convert Markdown to standalone HTML5 documents"
  homepage "https://maphew.github.io/mdo/"
  version "0.4.0"
  license any_of: ["MIT", "Apache-2.0"]

  on_macos do
    url "https://github.com/maphew/mdo/releases/download/v0.4.0/mdo-universal-apple-darwin.tar.gz"
    sha256 "11e029412635767ca2328410f02ea962238274392a61352012694eb7eae4e6f0"
  end

  on_linux do
    on_intel do
      url "https://github.com/maphew/mdo/releases/download/v0.4.0/mdo-x86_64-unknown-linux-gnu.tar.gz"
      sha256 "a8e5c8037056f94ed560e7a146579167d16fcb7efa54b05b6ab2911e297f7b6c"
    end
  end

  def install
    bin.install "mdo"
    bin.install "mdo-open"
    bin.install "mdo-setup" if OS.linux?
    doc.install "CHANGELOG.md", "README.md"
    license.install "LICENSE-APACHE", "LICENSE-MIT"
  end

  test do
    (testpath/"input.md").write("# Hello\n")
    system bin/"mdo", testpath/"input.md", "--output", testpath/"output.html"
    assert_match "<h1>Hello</h1>", (testpath/"output.html").read
  end
end
