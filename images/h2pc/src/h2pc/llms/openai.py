from typing import Iterable, final, override

from openai import AsyncClient
from openai.types import chat
from pydantic import SecretStr

from .base import BaseLLM
from ..models.message import Message


def _encode_messages(
    messages: Iterable[Message],
) -> Iterable[chat.ChatCompletionMessageParam]:
    return (
        message.build_openai()
        for message in messages
    )


class OpenAILLM(BaseLLM):
    '''
    An OpenAI LLM class.
    '''

    def __init__(
        self,
        api_key: SecretStr,
        base_url: str,
        model_name: str,
    ) -> None:
        super().__init__(model_name)
        self._client = AsyncClient(
            api_key=api_key.get_secret_value(),
            base_url=base_url,
        )

    @final
    @override
    async def completions(self, inputs: list[Message]) -> Message | None:
        completion = await self._client.chat.completions.create(
            messages=_encode_messages(inputs),
            model=self._model_name,
        )
        content = completion.choices[0].message.content
        if content is not None:
            return Message(
                role='assistant',
                content=content,
            )
        return None
