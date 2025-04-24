from abc import ABCMeta, abstractmethod
import asyncio
from pathlib import Path
from typing import final

from loguru import logger
from pydantic import BaseModel

from .._metadata.version import __version__
from ..messengers.base import BaseMessenger, MessengerBaseSettings
from ..messengers.kafka import KafkaMessenger
from ..messengers.nats import NatsMessenger


class AgentBaseSettings(MessengerBaseSettings):
    '''
    A base agent settings model class.
    '''

    prompt_path: Path
    verbose: bool = False


class AgentBase[
    S: AgentBaseSettings,
    I: BaseModel,
    O: BaseModel,
](metaclass=ABCMeta):
    '''
    A virtual agent base class.
    '''

    def __init__(self) -> None:
        self._logger = logger
        self._logger.info(f'Welcome to the H2PC {self.name} ({self.version})!')
        self._settings = self._settings_type()  # type: ignore

        messenger_type: type[BaseMessenger]
        match self.settings.messenger_kind:
            case 'kafka':
                messenger_type = KafkaMessenger
            case 'nats':
                messenger_type = NatsMessenger
        self._messenger = messenger_type(
            settings=self.settings,
            input_type=self._input_type,
        )

    @property
    def name(self) -> str:
        '''
        Return the agent name.
        '''
        return type(self).__name__.title()

    @property
    def version(self) -> str:
        '''
        Return the agent package version.
        '''
        return __version__

    @property
    @abstractmethod
    def _settings_type(cls) -> type[S]:
        '''
        Return the settings class type.
        '''
        pass

    @property
    @abstractmethod
    def _input_type(cls) -> type[I]:
        '''
        Return the input data class type.
        '''
        pass

    @final
    @property
    def settings(self) -> S:
        '''
        Return the settings.
        '''
        return self._settings

    @final
    async def exec(self) -> None:
        '''
        Consume the process and execute the agent task.
        '''
        self._logger.info('Started agent')
        try:
            while True:
                inputs = await self.consume_batch()
                for _ in range(self.settings.messenger_sink_count_min):
                    new_output = await self.process(inputs)
                    if new_output is None:
                        break
                    await self.produce(new_output)
                await self.flush()
        except asyncio.exceptions.CancelledError:
            self._logger.info('CTRL+C captured')
        finally:
            self._logger.info('Terminating')
            return await self.terminate()

    @final
    async def consume(self) -> I | None:
        '''
        Read a data.
        '''
        return await self._messenger.consume()

    @final
    async def consume_batch(self) -> list[I]:
        '''
        Read a batch of data.
        '''
        return await self._messenger.consume_batch()

    @final
    async def produce(self, data: O) -> None:
        '''
        Write the data.
        '''
        return await self._messenger.produce(data)

    @final
    async def flush(self) -> None:
        '''
        Flush the data.
        '''
        return await self._messenger.flush()

    @abstractmethod
    async def process(self, data: list[I]) -> O | None:
        '''
        Process the data.
        '''
        pass

    async def terminate(self) -> None:
        '''
        Terminate the agent.
        '''
        return await self._messenger.terminate()
