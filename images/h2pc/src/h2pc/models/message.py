from typing import Literal

from ollama import Message as OllamaMessage
from openai.types import chat as openai_chat
from pydantic import BaseModel


class Message(BaseModel):
    role: Literal['assistant', 'system', 'user']
    '''The role of the messages author.'''

    name: str | None = None
    '''The optional name for the participant.'''

    content: str
    '''The content of the message.'''

    def build_ollama(self) -> OllamaMessage:
        return OllamaMessage(
            role=self.role,
            content=self.content,
        )

    def build_openai(self) -> openai_chat.ChatCompletionMessageParam:
        match self.role:
            case 'assistant':
                return openai_chat.ChatCompletionAssistantMessageParam(
                    role='assistant',
                    name=self.name or '',
                    content=self.content,
                )
            case 'system':
                return openai_chat.ChatCompletionSystemMessageParam(
                    role='system',
                    name=self.name or '',
                    content=self.content,
                )
            case 'user':
                return openai_chat.ChatCompletionUserMessageParam(
                    role='user',
                    name=self.name or '',
                    content=self.content,
                )
