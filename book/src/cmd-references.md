# 命令介绍

`cldi`是对[CITA-Cloud协议](https://github.com/cita-cloud/cita_cloud_proto)定义的对外的gRPC接口的封装。

这里列出这些接口以供参考。

## controller的接口

[controller proto](https://github.com/cita-cloud/cita_cloud_proto/blob/master/protos/controller.proto#L53-L90)

```protobuf
service RPCService {
    // flag means latest or pending.
    // true means pending, false means latest.
    rpc GetBlockNumber(Flag) returns (BlockNumber);

    rpc SendRawTransaction(blockchain.RawTransaction) returns (common.Hash);

    rpc SendRawTransactions(blockchain.RawTransactions) returns (common.Hashes);

    rpc GetBlockByHash(common.Hash) returns (blockchain.CompactBlock);

    rpc GetHeightByHash(common.Hash) returns (BlockNumber);

    rpc GetBlockByNumber(BlockNumber) returns (blockchain.CompactBlock);

    rpc GetStateRootByNumber (BlockNumber) returns (common.StateRoot);

    rpc GetProofByNumber (BlockNumber) returns (common.Proof);

    rpc GetBlockDetailByNumber(BlockNumber) returns (blockchain.Block);

    rpc GetTransaction(common.Hash) returns (blockchain.RawTransaction);

    rpc GetSystemConfig(common.Empty) returns (SystemConfig);

    rpc GetSystemConfigByNumber(BlockNumber) returns (SystemConfig);

    rpc GetBlockHash(BlockNumber) returns (common.Hash);

    rpc GetTransactionBlockNumber(common.Hash) returns (BlockNumber);

    rpc GetTransactionIndex(common.Hash) returns (TransactionIndex);

    // add new node
    rpc AddNode(common.NodeNetInfo) returns (common.StatusCode);

    rpc GetNodeStatus(common.Empty) returns (common.NodeStatus);
}
```

## executor的接口
[executor proto](https://github.com/cita-cloud/cita_cloud_proto/blob/master/protos/executor.proto#L41-L46)

其中call的数据格式由具体的executor微服务定义。
```protobuf
service ExecutorService {
    // exec a block return executed_block_hash
    rpc Exec(blockchain.Block) returns (common.HashResponse);

    rpc Call(CallRequest) returns (CallResponse);
}
```

## executor_evm的接口
[executor_evm proto](https://github.com/cita-cloud/cita_cloud_proto/blob/master/protos/vm/evm.proto#L69-L81)
```protobuf
service RPCService {
  rpc GetTransactionReceipt(common.Hash) returns (Receipt);

  rpc GetCode(common.Address) returns (ByteCode);

  rpc GetBalance(common.Address) returns (Balance);

  rpc GetTransactionCount(common.Address) returns (Nonce);

  rpc GetAbi(common.Address) returns (ByteAbi);

  rpc EstimateQuota(executor.CallRequest) returns (ByteQuota);
}
```
