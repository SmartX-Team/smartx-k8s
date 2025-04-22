#!/usr/bin/env python

from abc import ABCMeta, abstractmethod
import asyncio
import logging
from pathlib import Path
from typing import Iterable, Literal, final, override

from jinja2 import Template
from kafka import KafkaConsumer, KafkaProducer
from nats.aio.client import Client as Nats
from nats.js.client import JetStreamContext
from ollama import Client as OllamaClient, Message as OllamaMessage
from openai import Client as OpenAIClient
from openai.types import chat as openai_chat
from pydantic import BaseModel, SecretStr
from pydantic_settings import BaseSettings, SettingsConfigDict
import yaml

# Load environment variables from `.env` file
try:
    from dotenv import load_dotenv
    load_dotenv()
except ImportError:
    pass


class AgentBaseSettings(BaseSettings):
    '''
    A base agent settings model class.
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

    prompt_path: Path
    verbose: bool = False

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


class AgentBase[S: AgentBaseSettings, I, O](metaclass=ABCMeta):
    '''
    A virtual agent base class.
    '''

    def __init__(self) -> None:
        self._logger = logging.getLogger(__name__)
        logging.basicConfig(
            encoding='utf-8',
            level=logging.INFO,
        )
        self._logger.info(f'Welcome to the {self.name}!')
        self._settings = self._settings_type()()  # type: ignore

    @classmethod
    @abstractmethod
    def _settings_type(cls) -> type[S]:
        '''
        Return the settings class type.
        '''
        pass

    @property
    @abstractmethod
    def name(self) -> str:
        '''
        Return the agent name.
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
                inputs: list[I] = []
                while len(inputs) < self.settings.messenger_src_count_min:
                    new_input = await self._consume()
                    if new_input is None:
                        break
                    inputs.append(new_input)

                for _ in range(self.settings.messenger_sink_count_min):
                    new_output = await self._process(inputs)
                    if new_output is None:
                        break
                    await self._produce(new_output)
        except KeyboardInterrupt:
            self._logger.info('CTRL+C captured')
            return await self.terminate()

    async def _consume(self) -> I | None:
        '''
        Read a data.
        '''
        return None

    @abstractmethod
    async def _process(self, data: list[I]) -> O | None:
        '''
        Process the data.
        '''
        pass

    async def _produce(self, data: O) -> None:
        '''
        Write the data.
        '''
        pass

    async def terminate(self) -> None:
        '''
        Terminate the agent.
        '''
        pass


class AgentSettings(AgentBaseSettings):
    ollama_base_url: str | None = None
    ollama_model_name: str | None = None

    openai_api_key: SecretStr | None = None
    openai_base_url: str = 'https://api.openai.com'
    openai_model_name: str | None = None


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


class PromptTemplate(BaseModel):
    operator: Literal['Index', 'Message'] = 'Message'
    inputs: list[Message]


class Agent(AgentBase[AgentSettings, Message, Message]):
    def __init__(self) -> None:
        super().__init__()

        self._bootstrapper_servers = \
            self.settings.messenger_bootstrap_servers_list()
        self._topic_src = self.settings.messenger_topic_src_list()
        self._topic_sink = self.settings.messenger_topic_sink_list()

        # NATS
        if self.settings.messenger_kind == 'nats':
            self._nc = Nats()
        else:
            self._nc = None
        self._js: JetStreamContext | None = None

        # self._consumer
        if self._topic_src:
            match self.settings.messenger_kind:
                case 'kafka':
                    options = {}
                    if self.settings.messenger_client_id is not None:
                        options['client_id'] = \
                            self.settings.messenger_client_id
                    if self.settings.messenger_group_id is not None:
                        options['group_id'] = self.settings.messenger_group_id
                    self._consumer = KafkaConsumer(
                        allow_auto_create_topics=False,
                        auto_offset_reset='latest',
                        bootstrap_servers=self._bootstrapper_servers,
                        metrics_enabled=True,
                        **options,
                    )
                    self._consumer.subscribe(self._topic_src)
                    assert self._consumer.bootstrap_connected()
                case 'nats':
                    self._consumer = self._nc
        else:
            self._consumer = None

        # self._producer
        if self._topic_sink:
            match self.settings.messenger_kind:
                case 'kafka':
                    options = {}
                    if self.settings.messenger_client_id is not None:
                        options['client_id'] = \
                            self.settings.messenger_client_id
                    self._producer = KafkaProducer(
                        allow_auto_create_topics=False,
                        bootstrap_servers=self._bootstrapper_servers,
                        client_id=self.settings.messenger_client_id,
                        compression_type=None,
                        metrics_enabled=True,
                        **options,
                    )
                case 'nats':
                    self._producer = self._nc
        else:
            self._producer = None

        # self._llm
        if self.settings.ollama_model_name is not None:
            self._llm = OllamaClient(
                host=self.settings.ollama_base_url,
            )
        elif self.settings.openai_api_key is not None and \
                self.settings.openai_model_name is not None:
            self._llm = OpenAIClient(
                api_key=self.settings.openai_api_key.get_secret_value(),
                base_url=self.settings.openai_base_url,
            )
        else:
            self._llm = None

        self._prompt: Template | None
        if self._llm is not None:
            with open(
                file=self.settings.prompt_path,
                encoding='utf-8',
            ) as fp:
                self._prompt = Template(fp.read())
        else:
            self._prompt = None

    @property
    @final
    @override
    def name(self) -> str:
        return 'H2PC LLM Agent'

    @classmethod
    @final
    @override
    def _settings_type(cls) -> type[AgentSettings]:
        return AgentSettings

    @final
    async def _nats(self) -> JetStreamContext:
        if self._nc is None:
            raise ValueError('Invalid access to NATS client')
        if not self._nc.is_connected:
            await self._nc.connect(
                servers=self._bootstrapper_servers,
            )

        if self._js is None:
            self._js = self._nc.jetstream()
            if self._topic_sink:
                await self._js.add_stream(
                    name=self.settings.messenger_group_id,
                    subjects=self._topic_sink,
                )
        return self._js

    @final
    @override
    async def _consume(self) -> Message | None:
        if isinstance(self._consumer, KafkaConsumer):
            data: bytes = next(self._consumer).value
        elif isinstance(self._consumer, Nats):
            js = await self._nats()
            self._consumer = await js.subscribe(
                subject=self._topic_src[0],
                queue=self.settings.messenger_group_id,
                durable=self.settings.messenger_group_id,
            )
            return await self._consume()
        elif isinstance(self._consumer, JetStreamContext.PushSubscription):
            msg = await self._consumer.next_msg(timeout=None)
            await msg.ack()
            data = msg.data
        elif self._consumer is None:
            return None
        else:
            raise ValueError('Unknown consumer')
        return Message.model_validate_json(data)

    @final
    def _build_prompt_ollama(
        self,
        messages: Iterable[Message],
    ) -> Iterable[OllamaMessage]:
        return (
            message.build_ollama()
            for message in messages
        )

    @final
    def _build_prompt_openai(
        self,
        messages: Iterable[Message],
    ) -> Iterable[openai_chat.ChatCompletionMessageParam]:
        return (
            message.build_openai()
            for message in messages
        )

    @final
    def _build_prompt_template(
        self,
        data: list[Message],
    ) -> PromptTemplate:
        if self._prompt is None:
            raise Exception('Empty prompt')

        return PromptTemplate.model_validate(yaml.safe_load(
            stream=self._prompt.render(
                inputs=data,
            )
        ))

    @final
    @override
    async def _process(self, data: list[Message]) -> Message | None:
        template = self._build_prompt_template(data)

        # Generate content
        content: str | None
        if isinstance(self._llm, OllamaClient):
            completion = self._llm.chat(
                model=self.settings.ollama_model_name or '',  # type: ignore
                messages=list(self._build_prompt_ollama(template.inputs)),
            )
            content = completion.message.content
        elif isinstance(self._llm, OpenAIClient):
            completion = self._llm.chat.completions.create(
                model=self.settings.openai_model_name,  # type: ignore
                messages=self._build_prompt_openai(template.inputs),
            )
            content = completion.choices[0].message.content
        else:
            raise ValueError('Unknown LLM client')

        # Validate output content
        if content is None:
            return None
        if self.settings.verbose:
            logging.info(f'Generated: {content}')

        # Filter content
        content = content.strip()
        if content.startswith('<thought>'):
            content = content.split('</thought>')[-1].strip()

        # Parse content
        match template.operator:
            case 'Index':
                index = int(content)
                if index < 1 or index >= len(data):
                    return None
                return data[index - 1]
            case 'Message':
                return Message(
                    role='assistant',
                    content=content,
                )

    @final
    @override
    async def _produce(self, data: Message | None) -> None:
        if data is not None:
            value = data.model_dump_json().encode('utf-8')
            for topic in self._topic_sink:
                if isinstance(self._producer, KafkaProducer):
                    self._producer.send(topic, value)
                elif isinstance(self._producer, Nats):
                    js = await self._nats()
                    await js.publish(
                        subject=topic,
                        payload=value,
                    )
                elif self._producer is None:
                    return
                else:
                    raise ValueError('Unknown producer')

            # Flush
            if isinstance(self._producer, KafkaProducer):
                self._producer.flush()
            elif isinstance(self._producer, Nats):
                await self._producer.flush()
            elif self._producer is None:
                return
            else:
                raise ValueError('Unknown producer')

    @final
    @override
    async def terminate(self) -> None:
        if isinstance(self._consumer, KafkaConsumer):
            self._consumer.close()
        elif isinstance(self._consumer, Nats):
            await self._consumer.close()
        elif isinstance(self._consumer, JetStreamContext.PushSubscription):
            await self._consumer.unsubscribe()

        if isinstance(self._producer, KafkaConsumer):
            self._producer.close()
        elif isinstance(self._producer, Nats):
            await self._producer.close()


async def main() -> None:
    agent = Agent()
    return await agent.exec()

if __name__ == '__main__':
    asyncio.run(main())
