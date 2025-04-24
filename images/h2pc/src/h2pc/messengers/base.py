from abc import ABCMeta, abstractmethod
from typing import Literal, final, override

from pydantic import BaseModel
from pydantic_settings import BaseSettings, SettingsConfigDict


class MessengerBaseSettings(BaseSettings):
    '''
    A base messenger settings model class.
    '''

    model_config = SettingsConfigDict(
        env_file='.env',
        env_file_encoding='utf-8',
        extra='ignore',
    )

    messenger_bootstrap_servers: str | None = None
    messenger_client_id: str | None = None
    messenger_group_id: str | None = None
    messenger_kind: Literal['kafka', 'nats']
    messenger_src_count_min: int = 1
    messenger_src_count_max: int | None = None
    messenger_sink_count_min: int = 1
    messenger_sink_count_max: int | None = None
    messenger_topic_src: str | None = None
    messenger_topic_sink: str | None = None

    def messenger_bootstrap_servers_list(self) -> list[str]:
        return self.messenger_bootstrap_servers.split(',') \
            if self.messenger_bootstrap_servers \
            else []

    def messenger_topic_src_list(self) -> list[str]:
        return self.messenger_topic_src.split(',') \
            if self.messenger_topic_src \
            else []

    def messenger_topic_sink_list(self) -> list[str]:
        return self.messenger_topic_sink.split(',') \
            if self.messenger_topic_sink \
            else []


class BaseMessenger[
    I: BaseModel,
    O: BaseModel,
](metaclass=ABCMeta):
    '''
    A virtual messenger base class.
    '''

    def __init__(
        self,
        settings: MessengerBaseSettings,
        input_type: type[I],
    ) -> None:
        self._bootstrapper_servers = \
            settings.messenger_bootstrap_servers_list()
        self._input_type = input_type
        self._src_count_min = settings.messenger_src_count_min
        self._topic_src = settings.messenger_topic_src_list()
        self._topic_sink = settings.messenger_topic_sink_list()

    @abstractmethod
    async def consume(self) -> I | None:
        '''
        Read a data.
        '''
        pass

    @final
    async def consume_batch(self) -> list[I]:
        '''
        Read a batch of data.
        '''
        inputs: list[I] = []
        while len(inputs) < self._src_count_min:
            new_input = await self.consume()
            if new_input is None:
                break
            inputs.append(new_input)
        return inputs

    @abstractmethod
    async def produce(self, data: O) -> None:
        '''
        Write the data.
        '''
        pass

    @abstractmethod
    async def flush(self) -> None:
        '''
        Flush the data.
        '''
        pass

    @abstractmethod
    async def terminate(self) -> None:
        '''
        Terminate the messenger.
        '''
        pass


class BinaryMessenger[
    I: BaseModel,
    O: BaseModel,
](BaseMessenger, metaclass=ABCMeta):
    '''
    A virtual binary messenger base class.
    '''

    @final
    @override
    async def consume(self) -> I | None:
        if self._topic_src:
            value = await self.consume_bytes()
            return self._input_type.model_validate_json(value)
        return None

    @abstractmethod
    async def consume_bytes(self) -> bytes:
        '''
        Read a data as bytes.
        '''
        pass

    @final
    @override
    async def produce(self, data: O) -> None:
        if self._topic_sink:
            value = data.model_dump_json().encode('utf-8')
            return await self.produce_bytes(value)
        return None

    @abstractmethod
    async def produce_bytes(self, data: bytes) -> None:
        '''
        Write the data as bytes.
        '''
        pass
