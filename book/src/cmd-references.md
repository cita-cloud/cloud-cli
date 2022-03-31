# 命令介绍

`cldi`是对[CITA-Cloud协议](https://github.com/cita-cloud/cita_cloud_proto)定义的对外的gRPC接口的封装。

这里列出这些接口以供参考。

## controller的接口

[controller proto](https://github.com/cita-cloud/cita_cloud_proto/blob/c6431a69bf6078e20295fe0742a81dec3d687917/protos/controller.proto#L57-L89)

```protobuf
service RPCService {
    // flag means latest or pending.
    // true means pending, false means latest.
    rpc GetBlockNumber(Flag) returns (BlockNumber);

    rpc SendRawTransaction(blockchain.RawTransaction) returns (common.Hash);

    rpc SendRawTransactions(blockchain.RawTransactions) returns (common.Hashes);

    rpc GetBlockByHash(common.Hash) returns (blockchain.CompactBlock);

    rpc GetBlockByNumber(BlockNumber) returns (blockchain.CompactBlock);

    rpc GetTransaction(common.Hash) returns (blockchain.RawTransaction);

    rpc GetSystemConfig(common.Empty) returns (SystemConfig);

    rpc GetVersion(common.Empty) returns (SoftwareVersion);

    rpc GetBlockHash(BlockNumber) returns (common.Hash);

    rpc GetTransactionBlockNumber(common.Hash) returns (BlockNumber);

    rpc GetTransactionIndex(common.Hash) returns (TransactionIndex);

    rpc GetPeerCount(common.Empty) returns (PeerCount);

    // add new node
    rpc AddNode(common.NodeNetInfo) returns (common.StatusCode);

    // get peers info
    rpc GetPeersInfo(common.Empty) returns (common.TotalNodeInfo);
}
```

## executor的接口
[executor proto](https://github.com/cita-cloud/cita_cloud_proto/blob/c6431a69bf6078e20295fe0742a81dec3d687917/protos/executor.proto#L39-L44)

其中call的数据格式由具体的executor微服务定义。
```protobuf
service ExecutorService {
    // exec a block return executed_block_hash
    rpc Exec(blockchain.Block) returns (common.HashResponse);

    rpc Call(CallRequest) returns (CallResponse);
}
```

## executor_evm的接口
[executor_evm proto](https://github.com/cita-cloud/cita_cloud_proto/blob/c6431a69bf6078e20295fe0742a81dec3d687917/protos/vm/evm.proto#L64-L74)
```protobuf
service RPCService {
  rpc GetTransactionReceipt(common.Hash) returns (Receipt);

  rpc GetCode(common.Address) returns (ByteCode);

  rpc GetBalance(common.Address) returns (Balance);

  rpc GetTransactionCount(common.Address) returns (Nonce);

  rpc GetAbi(common.Address) returns (ByteAbi);
}
```
