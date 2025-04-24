from typing import final, override
from kafka import KafkaConsumer, KafkaProducer
from pydantic import BaseModel

from .base import BinaryMessenger, MessengerBaseSettings


class KafkaMessenger[
    I: BaseModel,
    O: BaseModel,
](BinaryMessenger[I, O]):
    '''
    A kafka messenger class.
    '''

    def __init__(
        self,
        settings: MessengerBaseSettings,
        input_type: type[I],
    ) -> None:
        super().__init__(settings, input_type)

        # Consumer
        options = {}
        if settings.messenger_client_id is not None:
            options['client_id'] = settings.messenger_client_id
        if settings.messenger_group_id is not None:
            options['group_id'] = settings.messenger_group_id
        self._consumer = KafkaConsumer(
            allow_auto_create_topics=False,
            auto_offset_reset='latest',
            bootstrap_servers=self._bootstrapper_servers,
            metrics_enabled=True,
            **options,
        )
        self._consumer.subscribe(self._topic_src)
        assert self._consumer.bootstrap_connected()

        # Producer
        options = {}
        if settings.messenger_client_id is not None:
            options['client_id'] = settings.messenger_client_id
        self._producer = KafkaProducer(
            allow_auto_create_topics=False,
            bootstrap_servers=self._bootstrapper_servers,
            client_id=settings.messenger_client_id,
            compression_type=None,
            metrics_enabled=True,
            **options,
        )

    @final
    @override
    async def consume_bytes(self) -> bytes:
        return next(self._consumer).value

    @final
    @override
    async def produce_bytes(self, data: bytes) -> None:
        for topic in self._topic_sink:
            self._producer.send(topic, data)

    @final
    @override
    async def flush(self) -> None:
        return self._producer.flush()

    @final
    @override
    async def terminate(self) -> None:
        if self._consumer.bootstrap_connected():
            self._consumer.close()
        if self._producer.bootstrap_connected():
            self._producer.close()
