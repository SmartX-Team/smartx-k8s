from typing import Iterable, final, override

from ollama import AsyncClient, Message as NativeMessage

from .base import BaseLLM
from ..models.message import Message


def _encode_messages(
    messages: Iterable[Message],
) -> Iterable[NativeMessage]:
    return (
        message.build_ollama()
        for message in messages
    )


class OllamaLLM(BaseLLM):
    '''
    An Ollama LLM class.
    '''

    def __init__(
        self,
        base_url: str,
        model_name: str,
    ) -> None:
        super().__init__(model_name)
        self._client = AsyncClient(
            base_url=base_url,
        )

    @final
    @override
    async def completions(self, inputs: list[Message]) -> Message | None:
        completion = await self._client.chat(
            messages=list(_encode_messages(inputs)),
            model=self._model_name,
        )
        content = completion.message.content
        if content is not None:
            return Message(
                role='assistant',
                content=content,
            )
        return None
