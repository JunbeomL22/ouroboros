#!/usr/bin/env python3
"""
Browser automation wrapper for Ouroboros pipeline.

This script wraps browser_use to integrate with the Ouroboros pipeline.
It reads a task from stdin and outputs in the ===HOW===, ===RESULT=== format.

Requirements:
    pip install browser-use langchain-anthropic

Environment:
    ANTHROPIC_API_KEY: Your Anthropic API key
"""

import asyncio
import sys
import os
from typing import Optional


async def run_browser_task(task: str) -> tuple[str, str]:
    """
    Execute a browser automation task using browser_use.

    Returns:
        Tuple of (how_description, result_description)
    """
    try:
        from browser_use import Agent
        from langchain_anthropic import ChatAnthropic
    except ImportError as e:
        return (
            f"Failed to import browser_use dependencies: {e}",
            "Error: browser_use not installed. Run: pip install browser-use langchain-anthropic"
        )

    # Check for API key
    api_key = os.environ.get("ANTHROPIC_API_KEY")
    if not api_key:
        return (
            "No ANTHROPIC_API_KEY found in environment",
            "Error: ANTHROPIC_API_KEY environment variable not set"
        )

    try:
        # Initialize the LLM
        llm = ChatAnthropic(
            model="claude-sonnet-4-20250514",
            api_key=api_key,
            timeout=120,
            stop=None
        )

        # Create browser_use agent
        agent = Agent(
            task=task,
            llm=llm,
        )

        # Run the agent
        result = await agent.run()

        # Format the output
        how = f"""Browser automation executed with browser_use agent.
- Task: {task[:200]}{'...' if len(task) > 200 else ''}
- Agent completed execution
- Actions performed: {len(result.history) if hasattr(result, 'history') else 'N/A'} steps"""

        result_text = f"""Browser automation completed.
- Final result: {str(result.final_result)[:500] if hasattr(result, 'final_result') else str(result)[:500]}
- Errors: {result.errors if hasattr(result, 'errors') and result.errors else 'None'}"""

        return (how, result_text)

    except Exception as e:
        return (
            f"Browser automation encountered an error during execution: {type(e).__name__}",
            f"Error: {str(e)}"
        )


def main():
    """Main entry point - reads from stdin, outputs in expected format."""
    # Read task from stdin
    task = sys.stdin.read().strip()

    if not task:
        print("===HOW===")
        print("No task provided via stdin")
        print("")
        print("===RESULT===")
        print("Error: Empty task received")
        return

    # Run the browser task
    how, result = asyncio.run(run_browser_task(task))

    # Output in the expected format
    print("===HOW===")
    print(how)
    print("")
    print("===RESULT===")
    print(result)


if __name__ == "__main__":
    main()
