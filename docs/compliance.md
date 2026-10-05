# EU Law and Bugaboo

Status: **Draft for review** · Last updated 2026-10-04 · Feature IDs refer to
[01-feature-list.md](01-feature-list.md) (mainly COMP, DIST, PRIV, AI).

> **Not legal advice.** This page explains the EU laws that are likely to matter for
> Bugaboo, in plain words, so that we can design for them now. Dates and facts are as we
> understand them in **October 2026**; some details are marked *to be confirmed in the
> legal review*. A lawyer must review this before the first public release.

---

## Two ways Bugaboo can reach customers

We don't know yet whether Bugaboo will be sold, served, or both. We design for **both**:

| Model      | What it means                                             | Our role (roughly)                               |
|------------|-----------------------------------------------------------|--------------------------------------------------|
| **Sold**   | The customer downloads Bugaboo and runs it on their own server (*self-hosted*). | Manufacturer of a software product.  |
| **Served** | We run Bugaboo for the customer in EU data centres (*hosted by Bugaboo*). | Service provider, and processor of the customer's data. Still the manufacturer of the software. |

Both models use the same code and the same server program (DIST-01). The served model gives
each customer their own isolated instance (DIST-02).

---

## 1. Cyber Resilience Act (CRA)

**What it is.** Regulation (EU) 2024/2847. It sets security rules for "products with
digital elements" (hardware and software) sold in the EU: secure design, vulnerability
handling, security updates for a support period, documentation and a CE mark.

**Does it apply to us?**

- **Sold:** yes. The downloadable server program is a product with digital elements once we
  offer it commercially. Free, non-commercial open source is exempt; open-source projects
  backed by a foundation or company ("stewards") have a lighter set of duties. Our license
  decision is still open.
- **Served:** a pure online service is generally *outside* the CRA (it is covered by NIS2
  instead). But because we also offer the same program for download, the CRA applies to that
  program anyway. *Whether parts of the served setup count as "remote data processing" of the
  product: to be confirmed in the legal review.*
- **Category:** Bugaboo is most likely in the **default category** (self-assessment, no
  external auditor). *To be confirmed*, because some listed product types (for example
  identity management) could be argued to touch our sign-in features.

**What it asks of us.**

- Design and ship securely, with secure defaults, and assess cybersecurity risks.
- Know every component inside: a **software bill of materials (SBOM)**.
- Handle vulnerabilities: a way to report them, fix them, publish advisories.
- **Report actively exploited vulnerabilities and severe incidents** to the authorities
  through ENISA's single reporting platform: early warning within **24 hours**, notification
  within **72 hours**, final report later (within 14 days after a fix for vulnerabilities,
  one month for incidents).
- Provide free security updates for a declared **support period** (normally at least
  5 years) and keep each security update available for a long time afterwards
  (*exact period: to be confirmed*).
- Keep technical documentation, sign an EU declaration of conformity, put a CE mark on the
  product (for software: in the documentation and download page).

**How Bugaboo meets it.** SBOM per release (COMP-01), third-party register (COMP-02),
vulnerability handling and reporting (COMP-03, DIST-08), secure defaults and support period
(COMP-04, SEC section), technical documentation file and conformity declaration (COMP-05),
signed updates (OPS-11, DIST-07).

**Key dates.** In force since 10 December 2024. **Reporting duties apply since
11 September 2026** (already now). All other duties apply from **11 December 2027**.

---

## 2. AI Act

**What it is.** Regulation (EU) 2024/1689. It sorts AI uses by risk: some are banned, some
are "high-risk" with heavy duties, and many only need transparency.

**Does it apply to us?** Yes, in both models, because Bugaboo includes an assistant built on
a general-purpose AI model (for example Claude). The model maker has its own duties. We are
most likely the **provider** of the assistant feature, and our customers are **deployers**
who use it at work. *Exact roles: to be confirmed in the legal review.*

**What it asks of us.**

- **Transparency** (Art. 50): people must know when they are talking to an AI, and
  AI-generated text must be marked, including in a machine-readable way.
- **AI literacy** (Art. 4): organizations using AI must make sure their staff understand it
  well enough. We help our customers with guidance.
- **Stay out of the banned and high-risk zones.** These would apply if we built them, so we
  don't:
  - **Banned:** recognising emotions of people at work. → Bugaboo never analyses the mood,
    tone or sentiment of people's comments or messages.
  - **High-risk:** AI that assigns work based on people's behaviour or personal traits, or
    that monitors and evaluates workers' performance. → The assistant never suggests *who*
    should work on something, never rates or ranks people, and never analyses an
    individual's working time.

**How Bugaboo meets it.** Labels on everything the assistant writes and a stored
"written with the assistant" mark that survives export (COMP-08, COMP-09); AI off by default
and data shown before sending (AI-02, PRIV-08); suggestions limited to type, labels and
priority (AI-14); hard product rules against people-related AI (COMP-10, PRIV-09); AI
literacy guide for admins and users (COMP-11).

**Key dates.** In force since 1 August 2024. Bans and AI literacy apply since
**2 February 2025**. Transparency rules apply from **2 August 2026**. High-risk rules for
the Annex III uses were also planned for 2 August 2026; *an EU proposal to delay them was
under discussion — to be confirmed*. We avoid those uses entirely, so the date doesn't change
our design.

---

## 3. NIS2

**What it is.** Directive (EU) 2022/2555, implemented by each EU country in national law. It
requires organisations in important sectors to manage cybersecurity risks, report serious
incidents, and make their management personally responsible for it.

**Does it apply to us?**

- **Served:** probably yes, **if we are medium-sized or larger** (roughly 50+ employees or
  more than €10 million turnover). Cloud computing services and managed service providers are
  listed sectors. A small company is usually outside, unless the national law says otherwise.
  *Company size and the German implementation status: to be confirmed in the legal review.*
- **Sold:** not directly. But many of our customers will be under NIS2, and NIS2 makes them
  check the security of their suppliers. They will ask us for evidence (SBOM, vulnerability
  handling, security features).

**What it asks of an organisation in scope.**

- Risk management measures: access control, encryption, backups, incident handling,
  supply-chain security, training, and more.
- **Incident reporting:** early warning within **24 hours**, notification within
  **72 hours**, final report within **one month**.
- Management approves and oversees the measures and can be held responsible.

**How Bugaboo meets it.** For us as operator: served-model security and runbooks (DIST-02,
DIST-03, DIST-08). For customers: security log, exports and incident timelines (SEC-15,
COMP-14), SBOM and vulnerability handling (COMP-01, COMP-03), strong sign-in and access
control (AUTH, SEC).

**Key dates.** Member states had to apply it from 18 October 2024. National laws vary;
*Germany's implementation law and its exact obligations: to be confirmed*.

---

## 4. Data Act

**What it is.** Regulation (EU) 2023/2854. Most of it is about data from connected devices,
which doesn't concern us. One part is about **switching between data processing services**:
customers must be able to leave a cloud service and take their data with them.

**Does it apply to us?**

- **Served:** yes, the switching rules apply to us as a provider of a data processing
  service.
- **Sold:** mostly no (we don't process the data). Easy export is still good practice.

**What it asks of us (served).**

- Let customers switch to another provider or to their own servers, with all their data and
  configuration, in open formats.
- Contracts must state notice periods (at most 2 months) and a transition period (normally
  30 days), and must not contain obstacles to switching.
- Switching charges: only at cost since 12 September 2025, **no charges at all from
  12 January 2027**.
- Publish where our infrastructure is and what protects data from unlawful access by
  non-EU governments.

**How Bugaboo meets it.** Full export and import, both ways, between served and self-hosted
(DIST-06, COMP-12); served-model paperwork including data location and exit terms
(DIST-05).

**Key dates.** Applies since **12 September 2025**; switching charges end
**12 January 2027**.

---

## 5. GDPR

**What it is.** Regulation (EU) 2016/679, the data protection law. It covers every piece of
personal data: names, e-mail addresses, comments, working hours, sign-in logs.

**Who is who.**

- **Sold:** the customer is the **controller**. We are not involved, because we have no
  access to their data. If they let our support look at their system, we act as their
  **processor** for that support case.
- **Served:** the customer is the controller; we are their **processor**. That requires a
  **data processing agreement** (Art. 28), a list of our **sub-processors** (for example the
  data centre, the e-mail service, an AI provider), the **technical and organisational
  measures** we take (Art. 32), and data kept in the EU.
- **Both:** for our own customer accounts and billing, we are the controller.

**How Bugaboo meets it.** The whole PRIV section (PRIV-01…11): privacy by default, data map,
download my data, remove a person, keep-and-delete rules, protection of employee data; plus
DIST-03 (no operator access by default) and DIST-05 (served-model paperwork).

**Key dates.** Applies since 25 May 2018.

---

## 6. ePrivacy (cookies)

**What it is.** The ePrivacy Directive, in Germany implemented by the TDDDG. Storing or
reading anything on a person's device needs consent, unless it is strictly necessary.

**Does it apply to us?** Yes, in both models.

**How Bugaboo meets it.** Only strictly necessary cookies and storage (staying signed in,
security), no tracking, no outside scripts or fonts. So no consent banner is needed
(PRIV-01, COMP-13).

The browser also keeps three interface preferences on the device, which only the person
chose and which hold no personal data: `bugaboo.appearance` (theme, spacing, text size, motion),
`bugaboo.shortcut-keys` (own keyboard shortcuts) and `bugaboo.sidebar-narrow` (menu narrowed to
icons). They are strictly necessary for the service the person asked for (a screen that looks and
works the way they set it), so they need no consent: TDDDG section 25 (2) no. 2.

---

## 7. Accessibility

**What it is.** The **European Accessibility Act** (Directive 2019/882; in Germany the BFSG)
requires certain consumer products and services to be accessible. The European standard
**EN 301 549** defines what "accessible" means for software, and it is required when selling
to the public sector.

**Does it apply to us?** Bugaboo is a tool for organisations, not a consumer service, so the
Accessibility Act most likely doesn't apply directly. *To be confirmed.* Public-sector
customers will require EN 301 549 anyway.

**How Bugaboo meets it.** WCAG 2.2 AA in every theme (UX-04), EN 301 549 and an
accessibility statement template (COMP-15).

**Key dates.** The Accessibility Act applies since **28 June 2025**.

---

## 8. Product Liability Directive (new)

**What it is.** Directive (EU) 2024/2853 replaces the old product liability rules and now
explicitly covers **software**. A manufacturer can be liable for damage caused by a defective
product, including missing security updates. Damage includes the destruction or corruption
of data (for private use).

**Does it apply to us?** Yes, for software placed on the market after the date below,
whether downloaded or provided as a service. Free open source outside a commercial activity
is excluded. *Details: to be confirmed.*

**How Bugaboo meets it.** Security updates for the support period (COMP-04), backups and
safe upgrades (OPS-06, OPS-07), release notes and known issues with every release (COMP-16).

**Key dates.** Applies to products placed on the market from **9 December 2026**.

---

## Who does what

| Law          | Sold (self-hosted): we …                                       | Sold: the customer …                                    | Served: we …                                                       | Served: the customer …                       |
|--------------|-----------------------------------------------------------------|---------------------------------------------------------|---------------------------------------------------------------------|----------------------------------------------|
| CRA          | Secure design, SBOM, vulnerability handling, reporting, updates, CE mark | Installs updates, runs it securely              | Same as sold, for the program                                       | —                                            |
| AI Act       | Transparency, marking, no people-related AI, provider information | Ensures AI literacy of staff, decides whether to switch the assistant on | Same as sold                                         | Ensures AI literacy, decides on the assistant |
| NIS2         | Give customers evidence (SBOM, security information)            | Own risk management and reporting, if in scope          | Risk management and incident reporting, if we are in scope         | Own duties, if in scope                      |
| Data Act     | — (export available)                                            | —                                                       | Switching, export, no switching charges, transparency on location   | Gives notice when switching                  |
| GDPR         | Privacy-friendly product; processor only for support access    | Controller: legal basis, privacy notice, records         | Processor: data processing agreement, sub-processors, security      | Controller                                   |
| ePrivacy     | Only necessary cookies                                          | —                                                       | Only necessary cookies                                              | —                                            |
| Accessibility| EN 301 549 conformance and statement                            | Publishes statement if public sector                    | Same as sold                                                        | Same as sold                                 |
| Liability    | Safe product, updates for the support period                    | Installs updates                                         | Safe service, updates applied by us                                 | —                                            |

---

## What this changes in Bugaboo

- **New:** section 22 *Sold and served* (DIST-01…08) in the feature list.
- **Reworked:** section 21 COMP (COMP-01…16), now tagged per model.
- **Changed:** AI-14 (the assistant no longer suggests who should work on an issue).
- **New non-goal:** no AI that evaluates, ranks or assigns work to people, and no emotion
  or mood analysis.
- **Related:** PRIV-01…11 (GDPR), SEC-15 (security log), SEC-18 (supply chain), OPS-11
  (signed updates), UX-04 (accessibility).

## Open points for the legal review

1. Business model, license and company size, which decide CRA scope (commercial or not)
   and NIS2 scope (size thresholds).
2. CRA product category and whether any served components count as part of the product.
3. Our exact role under the AI Act for the assistant (provider or deployer), and the final
   timeline for high-risk rules.
4. Germany's NIS2 implementation law and its requirements for us as a service provider.
5. Whether the Accessibility Act applies to any customer group we target.
6. Support period length and how long security updates stay downloadable.
