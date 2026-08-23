class Mdo < Formula
  desc "Convert Markdown to standalone HTML5 documents"
  homepage "https://maphew.github.io/mdo/"
  version "0.6.1"
  license any_of: ["MIT", "Apache-2.0"]

  on_macos do
    url "https://github.com/maphew/mdo/releases/download/v0.6.1/mdo-universal-apple-darwin.tar.gz"
    sha256 "f36cd3cf7153520e9d9875bf4e3e4a6ca164d1ab4fd948b3f91b930bd0c49a65"
  end

  on_linux do
    on_intel do
      url "https://github.com/maphew/mdo/releases/download/v0.6.1/mdo-x86_64-unknown-linux-gnu.tar.gz"
      sha256 "8848a297e47812751b84986f47c1719ae7600c7d4de33417eb227528c421f227"
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
