namespace SemanticGraph.Visualizer.Client.Services;

public sealed class GraphClientException : Exception
{
    public GraphClientException(string message)
        : base(message)
    {
    }
}
