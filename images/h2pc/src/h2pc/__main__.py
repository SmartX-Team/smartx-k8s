#!/usr/bin/env python
# Copyright (c) 2025 Ho Kim (ho.kim@ulagbulag.io). All rights reserved.
# Use of this source code is governed by a GPL-3-style license that can be
# found in the LICENSE file.

import asyncio

from dotenv import load_dotenv

from .agents.llm import LLMAgent


async def main() -> None:
    # Load environment variables from `.env` file
    load_dotenv()

    agent = LLMAgent()
    return await agent.exec()


if __name__ == '__main__':
    asyncio.run(main())
