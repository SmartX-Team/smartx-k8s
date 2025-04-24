from typing import Literal

from pydantic import BaseModel

from .message import Message


class PromptTemplate(BaseModel):
    operator: Literal['Index', 'Message'] = 'Message'
    inputs: list[Message]
