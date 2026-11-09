# 0.1.65 — redirect and canonical report comparison

The combined chain report includes both redirect and canonical starts. It includes multi-hop paths, loops and paths that reach the hop limit.

Non-Indexable Canonicals evaluates the direct canonical destination. It reports observed noindex directives, canonicalisation to another URL, redirects and response failures on that destination.

These changes came from a controlled ten-URL comparison with the installed Screaming Frog 25.1 release. Five of six reports match on source URL, destination and status. Headers and link provenance still differ. All Redirects retains individual redirect hops, including an extra intermediate hop on the test crawl. Windows acceptance also passed for the report picker and filtered UTF-8 CSV export.
