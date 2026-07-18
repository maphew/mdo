class Mdo < Formula
  desc "Convert Markdown to standalone HTML5 documents"
  homepage "https://maphew.github.io/mdo/"
  version "0.6.0"
  license any_of: ["MIT", "Apache-2.0"]

  on_macos do
    url "https://github.com/maphew/mdo/releases/download/v0.6.0/mdo-universal-apple-darwin.tar.gz"
    sha256 "45bdb4788084bee02390a37aa2d28030a7fb73a1c679f024a1c15bf48a39fb61"
  end

  on_linux do
    on_intel do
      url "https://github.com/maphew/mdo/releases/download/v0.6.0/mdo-x86_64-unknown-linux-gnu.tar.gz"
      sha256 "563458a5680c1ba473fa29ddc6d935805a0eb9bcdecbdde57a16d00b25f939ca"
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
