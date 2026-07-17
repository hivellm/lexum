# 1. Overview: History, Licensing, Ecosystem

> Part of the [Elasticsearch Analysis for Lexum](README.md). Findings are numbered globally across the analysis (F-001…).

## 1.1 History

- **2004** — Shay Banon writes **Compass**, a Java search library wrapping Apache Lucene. While designing Compass 3 he concludes a full rewrite is needed to get a scalable, distributed solution speaking JSON over HTTP ([Wikipedia](https://en.wikipedia.org/wiki/Elasticsearch)).
- **February 8, 2010** — First public release of Elasticsearch, announced with the tagline "You Know, for Search" ([Wikipedia](https://en.wikipedia.org/wiki/Elasticsearch)).
- **2012** — Elasticsearch BV (later **Elastic NV**) founded to commercialize the product.
- **October 2018** — Elastic IPOs on the NYSE as **ESTC** at $36/share; the stock roughly doubled on the first trading day ([CNBC](https://www.cnbc.com/2018/10/05/elastic-estc-ipo-stock-makes-debut-on-nyse.html), [GlobeNewswire](https://www.globenewswire.com/news-release/2018/10/04/1617209/0/en/Elastic-Announces-Pricing-of-Initial-Public-Offering.html)).
- **May 2019** — Core security (TLS, users, RBAC roles) becomes **free** in the default distribution as of 6.8.0 / 7.1.0 — previously a paid X-Pack feature ([Elastic blog](https://www.elastic.co/blog/security-for-elasticsearch-is-now-free)).
- **April 15, 2025** — **Elasticsearch 9.0** released, on **Apache Lucene 10** (better search/IO parallelism, sparse indexing) ([Elastic blog](https://www.elastic.co/blog/whats-new-elastic-platform-9-0-0), [Lucene 10 highlights](https://www.elastic.co/search-labs/blog/apache-lucene-10-release-highlights)).
- **2026** — Latest release line is **9.3.x**; actively supported versions are 9.3, 9.2, 9.1, 9.0, 8.19, 8.17 ([endoflife.date](https://endoflife.date/elasticsearch), [release notes](https://www.elastic.co/docs/release-notes/elasticsearch)).

Version milestones relevant to API design (details in later sections): 1.0 (2014, snapshots/aggregations), 2.0 (2015, filters merged into query context), 5.0 (2016, BM25 becomes default similarity, Painless scripting), 6.0 (2017, single mapping type per index), 7.0 (2019, new cluster coordination, types deprecated), 7.8 (composable index templates), 7.10 (PIT — point in time, last Apache-2.0 release), 7.11 (runtime fields beta, searchable snapshots GA), 8.0 (2022, types fully removed, security on by default, NLP/vector groundwork), 8.14 (2024, ES|QL GA), 8.16 (AGPL option), 9.0 (2025, Lucene 10, BBQ GA).

### F-001 — Elasticsearch is a 15+-year-old Lucene-based engine still evolving rapidly (9.3.x on Lucene 10 as of 2026)
- **Evidence:** [Wikipedia](https://en.wikipedia.org/wiki/Elasticsearch), [endoflife.date](https://endoflife.date/elasticsearch), [Elastic 9.0 blog](https://www.elastic.co/blog/whats-new-elastic-platform-9-0-0)
- **Impact:** Lexum inherits a mature, well-documented design space: 15 years of ES version milestones show which API decisions survived (BM25 default, single mapping type, composable templates) and which were reverted — a ready-made map of what to copy and what to avoid.
- **Confidence:** High

## 1.2 Licensing evolution

This is one of the most consequential stories in open-source infrastructure and a cautionary/strategic backdrop for any ES-compatible project:

| Period | License | Notes |
|---|---|---|
| 2010 – Jan 2021 | **Apache 2.0** | Fully open source. X-Pack (security, ML, alerting) was proprietary, later "open code" under the Elastic License. |
| Jan 2021 (7.11+) | **SSPL 1.0 / Elastic License v2** dual license | Explicitly aimed at preventing AWS from reselling Elasticsearch as a managed service. Not OSI-approved. |
| Aug 29, 2024 (shipped in 8.16) | **AGPLv3 added** as a third option alongside SSPL and ELv2 | "Elasticsearch is Open Source. Again!" — AGPLv3 is OSI-approved, so ES source is open source again; default binary distributions remain under the Elastic License ([Elastic blog](https://www.elastic.co/blog/elasticsearch-is-open-source-again), [Elastic press release](https://ir.elastic.co/news/news-details/2024/Elastic-Announces-Open-Source-License-for-Elasticsearch-and-Kibana-Source-Code/default.aspx), [InfoQ](https://www.infoq.com/news/2024/09/elastic-open-source-agpl/), [licensing FAQ](https://www.elastic.co/pricing/faq/licensing)). |

### F-002 — Elasticsearch's licensing arc (Apache 2.0 → SSPL/ELv2 → +AGPLv3) left the ecosystem wary and created durable space for permissively-licensed alternatives
- **Evidence:** [Elastic blog: Elasticsearch is Open Source. Again!](https://www.elastic.co/blog/elasticsearch-is-open-source-again), [InfoQ](https://www.infoq.com/news/2024/09/elastic-open-source-agpl/), [licensing FAQ](https://www.elastic.co/pricing/faq/licensing)
- **Impact:** The 2021 relicense (aimed at AWS) fractured the ecosystem and triggered the OpenSearch fork; even after the 2024 AGPL addition, default binary distributions remain under the Elastic License. A permissively-licensed, ES-compatible engine like Lexum has a genuine strategic opening.
- **Confidence:** High

## 1.3 The OpenSearch fork

- **April 2021** — AWS forks Elasticsearch 7.10.2 and Kibana 7.10 as **OpenSearch** / **OpenSearch Dashboards**, under Apache 2.0, in direct response to the license change ([Wikipedia](https://en.wikipedia.org/wiki/OpenSearch_(software)), [TechCrunch](https://techcrunch.com/2024/09/16/aws-brings-opensearch-under-the-linux-foundation-umbrella/)).
- **September 16, 2024** — AWS transfers OpenSearch to the **Linux Foundation** (OpenSearch Software Foundation), with premier members AWS, SAP, and Uber; by then it had 700M+ downloads and 200+ maintainers ([Linux Foundation](https://www.linuxfoundation.org/press/linux-foundation-announces-opensearch-software-foundation-to-foster-open-collaboration-in-search-and-analytics), [TechCrunch](https://techcrunch.com/2024/09/16/aws-brings-opensearch-under-the-linux-foundation-umbrella/)).

### F-003 — OpenSearch (fork of ES 7.10.2) is now a Linux Foundation project with 700M+ downloads and 200+ maintainers
- **Evidence:** [Linux Foundation announcement](https://www.linuxfoundation.org/press/linux-foundation-announces-opensearch-software-foundation-to-foster-open-collaboration-in-search-and-analytics), [TechCrunch](https://techcrunch.com/2024/09/16/aws-brings-opensearch-under-the-linux-foundation-umbrella/), [Wikipedia](https://en.wikipedia.org/wiki/OpenSearch_(software))
- **Impact:** The fork is permanent and vendor-neutral: two independent, heavily-used engines now implement the same 7.10-era API surface, which stabilizes that surface as a target.
- **Confidence:** High

### F-004 — The ES 7.10-era REST API is a de-facto open standard; compatibility with it buys Lexum the entire client/tooling ecosystem for free
- **Evidence:** [Wikipedia: OpenSearch](https://en.wikipedia.org/wiki/OpenSearch_(software)); ES and OpenSearch both implement the 7.10 surface, and the client/tooling ecosystem (language clients, Grafana, Logstash-compatible shippers, etc.) speaks it
- **Impact:** This is the single most important strategic fact for Lexum's API design: targeting the ES 7.10-era core surface (rather than the latest 9.x surface or a fully custom API) makes existing shippers, dashboards, and language clients work against Lexum with no per-tool integration effort.
- **Confidence:** High

## 1.4 Ecosystem (the "Elastic Stack")

- **Kibana** — visualization/administration UI: Discover, dashboards, Dev Tools console, alerting, ILM/index management UIs. Licensed like Elasticsearch (AGPL/SSPL/ELv2 as of 2024).
- **Logstash** — server-side ETL/ingest pipeline (inputs → filters → outputs).
- **Beats** — lightweight single-purpose shippers (Filebeat, Metricbeat, Packetbeat...), largely superseded by **Elastic Agent + Fleet**.
- **Language clients** — official clients for Java, JS, Python, .NET, Go, PHP, Ruby, Rust.
- Elastic's three commercial "solutions" built on the engine: **Search**, **Observability** (logs/metrics/APM), **Security** (SIEM/EDR). The observability and security businesses drove most of the engine's evolution after ~2017 (data streams, ILM, ES|QL, TSDB mode, frozen tier).

### F-005 — Elasticsearch won on the default-batteries ecosystem (shippers, UI, console), not only engine quality
- **Evidence:** The Elastic Stack composition above (Kibana, Logstash, Beats/Agent, official clients for 8 languages); observability/security solutions drove engine evolution after ~2017
- **Impact:** Lexum's planned GUI and MCP/UMICP protocol support are the analogous plays for the agent era: engine quality alone will not drive adoption — one-line integration paths will.
- **Confidence:** High

---

Next: [2. Architecture](02-architecture.md)
