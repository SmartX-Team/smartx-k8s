from abc import ABCMeta, abstractmethod

from ..models.message import Message


class BaseLLM(metaclass=ABCMeta):
    '''
    A virtual LLM base class.
    '''

    def __init__(
        self,
        model_name: str,
    ) -> None:
        self._model_name = model_name

    @abstractmethod
    async def completions(self, inputs: list[Message]) -> Message | None:
        pass
