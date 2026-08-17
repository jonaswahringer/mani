# Require a strict Generator Command output contract

A Generator Command writes only the final Markdown draft to stdout, writes progress and diagnostics to stderr, and exits zero on success. Mani rejects nonzero exits, empty output, invalid UTF-8, raw ANSI escapes, raw HTML, and other violations of the Custom Guide format before opening review; failure never changes the existing guide.
