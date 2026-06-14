using System.Net.Http.Json;
using System.Text.Json;
using SemanticGraph.Visualizer.Client.Models;

namespace SemanticGraph.Visualizer.Client.Services;

public sealed class GraphClient
{
    private static readonly JsonSerializerOptions JsonOptions = new(JsonSerializerDefaults.Web);

    private readonly HttpClient _httpClient;
    private long _nextRequestId;

    public GraphClient(HttpClient httpClient)
    {
        _httpClient = httpClient;
    }

    public Task<GraphProjectionDto> GetProjectionAsync(
        int limit,
        CancellationToken cancellationToken = default)
    {
        return SendAsync<GraphProjectionParamsDto, GraphProjectionDto>(
            "graph.projection",
            new GraphProjectionParamsDto(limit),
            cancellationToken);
    }

    public Task<GraphNodeDetailsDto> GetNodeDetailsAsync(
        string nodeId,
        CancellationToken cancellationToken = default)
    {
        return SendAsync<GraphNodeDetailsParamsDto, GraphNodeDetailsDto>(
            "graph.node_details",
            new GraphNodeDetailsParamsDto(nodeId),
            cancellationToken);
    }

    public Task<GraphEdgeDetailsDto> GetEdgeDetailsAsync(
        string edgeId,
        CancellationToken cancellationToken = default)
    {
        return SendAsync<GraphEdgeDetailsParamsDto, GraphEdgeDetailsDto>(
            "graph.edge_details",
            new GraphEdgeDetailsParamsDto(edgeId),
            cancellationToken);
    }

    public Task<GraphNodeSearchResultsDto> SearchNodesAsync(
        string query,
        int limit,
        CancellationToken cancellationToken = default)
    {
        return SendAsync<GraphSearchNodesParamsDto, GraphNodeSearchResultsDto>(
            "graph.search_nodes",
            new GraphSearchNodesParamsDto(query, limit),
            cancellationToken);
    }

    private async Task<TResult> SendAsync<TParams, TResult>(
        string method,
        TParams parameters,
        CancellationToken cancellationToken)
    {
        var request = new JsonRpcRequestDto<TParams>(
            "2.0",
            Interlocked.Increment(ref _nextRequestId),
            method,
            parameters);

        using var response = await _httpClient.PostAsJsonAsync(
            "rpc",
            request,
            JsonOptions,
            cancellationToken);

        if (!response.IsSuccessStatusCode)
        {
            throw new GraphClientException(
                $"backend returned HTTP {(int)response.StatusCode}");
        }

        var rpcResponse = await response.Content.ReadFromJsonAsync<JsonRpcResponseDto<TResult>>(
            JsonOptions,
            cancellationToken);

        if (rpcResponse is null)
        {
            throw new GraphClientException("backend returned an empty JSON-RPC response");
        }

        if (rpcResponse.Error is not null)
        {
            throw new GraphClientException(
                $"JSON-RPC {rpcResponse.Error.Code}: {rpcResponse.Error.Message}");
        }

        return rpcResponse.Result
            ?? throw new GraphClientException($"backend returned no result for {method}");
    }
}
