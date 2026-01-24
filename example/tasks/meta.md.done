# Meta Instructions: Web Browsing with browser_use

This document provides authoritative guidance for all Ouroboros pipeline roles when tasks require web browsing, web scraping, or internet research capabilities.

---

## Purpose and Objective

**Goal**: Enable `C:\Users\junbe\Projects\ouroboros\src\roles\actor.rs` to call `browser_use` when web browsing is sometimes necessary.

**Approach**: Efficiently and effectively integrate browser_use capabilities into the Actor role so that:
1. The Actor automatically detects when web browsing is required
2. The Actor invokes browser_use only when necessary (not for every task)
3. Resource usage is optimized by avoiding unnecessary browser operations

**Report Task**: Research and document a method to implement conditional browser_use invocation in the Actor role.

**Key Files**:
- Actor Role Implementation: `C:\Users\junbe\Projects\ouroboros\src\roles\actor.rs`
- Pipeline Orchestration: `C:\Users\junbe\Projects\ouroboros\src\pipeline.rs`
- Agent Subprocess Handler: `C:\Users\junbe\Projects\ouroboros\src\agent.rs`
- Configuration: `C:\Users\junbe\Projects\ouroboros\example\config.json`

**Output Locations**:
- Tasks Directory: `C:\Users\junbe\Projects\ouroboros\example\tasks\`
- Results Directory: `C:\Users\junbe\Projects\ouroboros\example\results\`
- Plans Directory: `C:\Users\junbe\Projects\ouroboros\example\plans\`
- How Files Directory: `C:\Users\junbe\Projects\ouroboros\example\hows\`

---

## 1. Project Context

### What This Enables
The `browser_use` Python package provides programmatic browser automation capabilities. When a task requires interacting with websites, the Actor MUST use browser_use to:
- Navigate to URLs and interact with web pages
- Extract content from websites (text, data, structured information)
- Fill forms, click buttons, and perform multi-step web interactions
- Handle dynamic JavaScript-rendered content that simple HTTP requests cannot access
- Take screenshots for verification when needed

### Why browser_use Over Alternatives
| Approach | Use When |
|----------|----------|
| `browser_use` | JavaScript-heavy sites, form interactions, dynamic content, authentication flows |
| `requests`/`httpx` | Simple REST APIs with well-defined endpoints, static file downloads |
| `WebFetch` tool | Quick content extraction from static pages without interaction |

**Default to browser_use** when in doubt about whether a site requires JavaScript rendering.

### Integration Point
The Actor role invokes browser_use via Python scripts. All upstream roles (Outliner, Advisor, Planner) must recognize web browsing requirements and plan accordingly. The Checker validates that browser_use was properly used when required.

---

## 2. Detecting When Web Browsing Is Required

### Explicit Triggers (MUST Use browser_use)
Any task containing these phrases requires browser_use:
- "browse to", "visit website", "navigate to URL"
- "scrape", "extract from website", "fetch web content"
- "research online", "find information on the web"
- "check website", "verify URL", "monitor webpage"
- "fill out form", "submit web form", "login to website"
- "interact with web application"
- "download from URL" (when authentication or JavaScript is involved)

### Implicit Triggers (Evaluate Case-by-Case)
Consider browser_use when:
- Gathering current/real-time information not available locally
- Comparing products, prices, or services from vendor websites
- Verifying external documentation that may have dynamic content
- Collecting data from multiple web sources
- Any task mentioning specific commercial websites or web applications

### When NOT to Use browser_use
- Reading local files or offline documentation
- Calling REST APIs with defined endpoints (use requests/httpx)
- Static file downloads without authentication
- Tasks explicitly stating "offline" or "local only"

---

## 3. Role-Specific Instructions

### 3.1 Outliner Role

When creating outlines for tasks involving web browsing:

**Structural Requirements:**
1. Explicitly flag which steps require browser_use under a "Web Browsing Phase" section
2. Separate concerns into distinct phases:
   - Data collection (browser_use execution)
   - Data processing (local computation)
   - Output generation (file writing)
3. Include a risk assessment covering:
   - Website unavailability or structural changes
   - Rate limiting or bot detection
   - Dynamic content loading delays
   - Authentication requirements

**Example Outline Structure:**
```
## Objectives
- Primary: [What specific information to gather]
- Deliverable: [Expected output format and location]

## Phase 1: Web Browsing (browser_use required)
- Target URLs: [List each URL explicitly]
- Data to extract: [Specific fields, elements, or content]
- Interaction steps: [Clicks, form fills, navigation needed]

## Phase 2: Data Processing
- Transform raw data into required format
- Validate extracted content

## Phase 3: Output
- Write to [absolute file path]
- Include source URLs and timestamps

## Risks
- [List potential failure modes]

## Success Criteria
- [Explicit pass/fail conditions]
```

### 3.2 Advisor Role

When reviewing outlines that involve web browsing:

**Critical Questions to Raise:**
1. Does the outline explicitly identify browser_use as the required tool?
2. Are specific URLs listed, or only vague domain references?
3. How will the Actor verify that scraping succeeded?
4. What happens if the target website structure differs from expectations?
5. Is there a simpler approach (public API, RSS feed, cached data)?
6. Are there rate limiting or bot detection concerns?
7. Is authentication required? If so, how will credentials be handled?

**Failure Mode Analysis:**
Challenge the outline on handling:
- Connection failures or timeouts
- Empty or unexpected page content
- CAPTCHA or bot detection
- Content behind login walls

**Do NOT approve** outlines that:
- Assume website structure without fallback handling
- Lack explicit URLs or vague about data extraction targets
- Missing error handling strategy

### 3.3 Planner Role

When creating detailed plans for web browsing tasks:

**Required Plan Elements:**

1. **Prerequisites Check:**
```
## Prerequisites
- Verify browser_use installed: pip show browser-use
- Verify langchain-anthropic installed: pip show langchain-anthropic
- Confirm ANTHROPIC_API_KEY is set
```

2. **Standard browser_use Pattern:**
```python
import asyncio
from browser_use import Agent
from langchain_anthropic import ChatAnthropic

async def execute_browser_task():
    llm = ChatAnthropic(model_name="claude-sonnet-4-20250514")
    agent = Agent(
        task="[SPECIFIC TASK DESCRIPTION - be precise about what to extract]",
        llm=llm
    )
    result = await agent.run()
    return result

result = asyncio.run(execute_browser_task())
```

3. **Step-by-Step Execution Plan:**
```
## Step 1: Create Python Script
- Create script at [ABSOLUTE PATH]
- Include browser_use initialization with exact task description

## Step 2: Execute Browsing
- Run the Python script
- Capture stdout/stderr for debugging

## Step 3: Validate Results
- Check that expected data was extracted
- Handle empty or partial results

## Step 4: Save Output
- Write to [ABSOLUTE PATH]
- Include: extracted data, source URLs, timestamp
- Format: [JSON/Markdown/CSV as specified]

## Step 5: Verification
- Confirm output file exists
- Validate content structure
```

4. **Error Handling Plan:**
```python
async def browse_with_retry(task_description, max_retries=3):
    llm = ChatAnthropic(model_name="claude-sonnet-4-20250514")

    for attempt in range(max_retries):
        try:
            agent = Agent(task=task_description, llm=llm)
            result = await agent.run()
            return {"success": True, "data": result, "attempt": attempt + 1}
        except Exception as e:
            if attempt == max_retries - 1:
                return {"success": False, "error": str(e), "attempts": max_retries}
            await asyncio.sleep(2 ** attempt)  # Exponential backoff

    return {"success": False, "error": "Max retries exceeded"}
```

### 3.4 Actor Role

**CRITICAL**: When the task or plan indicates web browsing is required, you MUST use browser_use. Do NOT substitute with other methods.

#### Standard Implementation Checklist

**Before Browsing:**
- [ ] Verify browser_use is installed: `pip show browser-use`
- [ ] Verify langchain-anthropic is installed: `pip show langchain-anthropic`
- [ ] Create Python script with proper async structure
- [ ] Define precise task string for the Agent (specific URLs, exact data to extract)

**During Execution:**
- [ ] Run the Python script
- [ ] Monitor for errors or timeouts
- [ ] Capture output for inclusion in ===RESULT===

**After Browsing:**
- [ ] Validate extracted data meets task requirements
- [ ] Save results to the EXACT path specified in the task
- [ ] Document the process in ===HOW=== section
- [ ] List output files with absolute paths in ===RESULT=== section

#### Complete Implementation Example

```python
import asyncio
import json
from datetime import datetime
from browser_use import Agent
from langchain_anthropic import ChatAnthropic

async def browse_and_extract():
    llm = ChatAnthropic(model_name="claude-sonnet-4-20250514")

    agent = Agent(
        task="Navigate to python.org and find the current stable Python version number. Return only the version string.",
        llm=llm
    )

    result = await agent.run()

    # Process and save results
    output = {
        "extracted_data": str(result),
        "source_url": "https://python.org",
        "extracted_at": datetime.now().isoformat()
    }

    return output

if __name__ == "__main__":
    result = asyncio.run(browse_and_extract())

    # Save to specified output path
    output_path = r"C:\Users\junbe\Projects\ouroboros\example\output\result.json"
    with open(output_path, "w", encoding="utf-8") as f:
        json.dump(result, f, indent=2)

    print(f"Output saved to: {output_path}")
```

#### ===HOW=== Section Requirements

When using browser_use, document:
- The exact task string passed to the Agent
- Whether browsing succeeded or failed (and why)
- Number of retries needed (if any)
- Key data points extracted
- Any unexpected behaviors or workarounds applied
- The Python script location (if persisted)

#### ===RESULT=== Section Requirements

Include:
- Absolute path to each output file created
- Summary of data collected (count of items, key fields present)
- Source URLs that were visited
- Timestamp of extraction
- Any failures or partial results with explanation

### 3.5 Checker Role

When validating tasks that required web browsing:

#### VERDICT: PASS Criteria
All of the following must be true:
- [ ] browser_use was used (evidence in HOW section or script exists)
- [ ] Output file exists at the specified absolute path
- [ ] Output format matches requirements (JSON, Markdown, CSV, etc.)
- [ ] Required data fields are present and populated
- [ ] Source URLs are documented
- [ ] Data appears to be real (not placeholder/mock values)

#### VERDICT: FAIL_MINOR Criteria
Pass on core functionality, but has cosmetic issues:
- Missing timestamps or metadata (when not explicitly required)
- Inconsistent formatting in output
- Missing comments in generated scripts
- Output file in slightly different location but accessible
- Extra unrequested data included (not harmful, just unnecessary)
- Minor typos in non-critical fields

#### VERDICT: FAIL_MAJOR Criteria
Any of the following triggers major failure:
- browser_use was NOT used when the task explicitly required web browsing
- No output file was created
- Output file is empty or contains only errors
- Output contains completely wrong or unrelated data
- Required information specified in task is missing
- Script errors that prevented any execution
- Hardcoded/mocked data when real web data was required
- Wrong file format (e.g., CSV when JSON was specified)

#### Checker Verification Process
1. Examine HOW section for evidence of browser_use execution
2. Verify output file exists at specified path
3. Validate output content against task requirements
4. Confirm data appears to be real (timestamps, varying values, etc.)
5. Check that source URLs are documented

### 3.6 Fixer Roles

#### MinorFixer (FAIL_MINOR only)
Allowed to fix:
- Add missing timestamps or metadata
- Correct formatting inconsistencies
- Add comments to scripts
- Fix typos
- Reorganize output structure

NOT allowed to:
- Re-run browser_use
- Modify extracted data
- Change output file locations
- Add missing core functionality

#### MajorFixer (FAIL_MAJOR)
Has authority to:
- Re-run browser_use with corrected parameters
- Create missing output files
- Fix script errors and re-execute
- Change the browsing approach if original failed
- Replace mock data with real browser_use execution

Should document in ===HOW===:
- What the original failure was
- How the fix addresses the failure
- Whether browser_use was re-executed

---

## 4. Example Tasks

### Example 1: Simple Data Extraction
```markdown
# Task: Get Current Python Version

Navigate to python.org and extract the current stable Python version.
Save to: C:\Users\junbe\Projects\ouroboros\example\output\python-version.txt
```

**Expected Actor behavior:** Use browser_use, extract version, save to exact path.

### Example 2: Multi-Page Research
```markdown
# Task: Compare Cloud Provider Pricing

Research basic tier VM pricing from AWS, Google Cloud, and Azure.
Create a comparison table in Markdown format.
Save to: C:\Users\junbe\Projects\ouroboros\example\output\cloud-pricing.md
```

**Expected Actor behavior:** Use browser_use to visit each provider's pricing page, extract data, compile into structured Markdown table with source URLs.

### Example 3: Form Interaction
```markdown
# Task: Search Python Documentation

Navigate to docs.python.org, use the search function to search for "asyncio",
and extract the first 5 result titles and URLs.
Save as JSON to: C:\Users\junbe\Projects\ouroboros\example\output\asyncio-docs.json
```

**Expected Actor behavior:** Use browser_use with task that includes form interaction, extract results, save as properly formatted JSON.

---

## 5. Constraints and Boundaries

### NEVER Do
- Use HTTP requests for sites that require JavaScript rendering
- Mock or fabricate web data
- Hardcode expected results instead of browsing
- Skip error handling for network operations
- Create output files in locations other than those specified in the task
- Assume website structure without fallback handling

### ALWAYS Do
- Use browser_use for any task requiring real web interaction
- Document all URLs visited in the output
- Save output to the EXACT absolute path specified in the task
- Include source attribution and timestamps in output
- Verify output file creation succeeded
- Handle failures gracefully with informative error messages

### Required Dependencies
```
pip install browser-use langchain-anthropic
```

Environment requirements:
- ANTHROPIC_API_KEY must be set
- Network connectivity required
- Sufficient disk space for output files

---

## 6. Troubleshooting Reference

| Issue | Cause | Solution |
|-------|-------|----------|
| ModuleNotFoundError: browser_use | Package not installed | Run: `pip install browser-use` |
| ModuleNotFoundError: langchain_anthropic | Package not installed | Run: `pip install langchain-anthropic` |
| "API key not found" | Missing environment variable | Set ANTHROPIC_API_KEY |
| Timeout errors | Slow site or complex task | Increase timeout, simplify task description |
| Empty results | Page structure unexpected | Refine task description, check URL accessibility |
| Rate limiting | Too many requests | Add delays, reduce request frequency |
| Bot detection | Site blocking automation | May need to try different approach or document limitation |

---

*This meta.md is the authoritative reference for web browsing tasks in the Ouroboros pipeline. All roles MUST consult these instructions when tasks involve internet access, web scraping, or online research.*
