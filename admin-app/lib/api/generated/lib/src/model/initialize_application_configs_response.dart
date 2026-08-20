//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:built_value/built_value.dart';
import 'package:built_value/serializer.dart';

part 'initialize_application_configs_response.g.dart';

/// InitializeApplicationConfigsResponse
///
/// Properties:
/// * [bindingId]
/// * [created]
/// * [fileCount]
/// * [status]
@BuiltValue()
abstract class InitializeApplicationConfigsResponse implements Built<InitializeApplicationConfigsResponse, InitializeApplicationConfigsResponseBuilder> {
  @BuiltValueField(wireName: r'binding_id')
  String get bindingId;

  @BuiltValueField(wireName: r'created')
  bool get created;

  @BuiltValueField(wireName: r'file_count')
  int get fileCount;

  @BuiltValueField(wireName: r'status')
  String get status;

  InitializeApplicationConfigsResponse._();

  factory InitializeApplicationConfigsResponse([void updates(InitializeApplicationConfigsResponseBuilder b)]) = _$InitializeApplicationConfigsResponse;

  @BuiltValueHook(initializeBuilder: true)
  static void _defaults(InitializeApplicationConfigsResponseBuilder b) => b;

  @BuiltValueSerializer(custom: true)
  static Serializer<InitializeApplicationConfigsResponse> get serializer => _$InitializeApplicationConfigsResponseSerializer();
}

class _$InitializeApplicationConfigsResponseSerializer implements PrimitiveSerializer<InitializeApplicationConfigsResponse> {
  @override
  final Iterable<Type> types = const [InitializeApplicationConfigsResponse, _$InitializeApplicationConfigsResponse];

  @override
  final String wireName = r'InitializeApplicationConfigsResponse';

  Iterable<Object?> _serializeProperties(
    Serializers serializers,
    InitializeApplicationConfigsResponse object, {
    FullType specifiedType = FullType.unspecified,
  }) sync* {
    yield r'binding_id';
    yield serializers.serialize(
      object.bindingId,
      specifiedType: const FullType(String),
    );
    yield r'created';
    yield serializers.serialize(
      object.created,
      specifiedType: const FullType(bool),
    );
    yield r'file_count';
    yield serializers.serialize(
      object.fileCount,
      specifiedType: const FullType(int),
    );
    yield r'status';
    yield serializers.serialize(
      object.status,
      specifiedType: const FullType(String),
    );
  }

  @override
  Object serialize(
    Serializers serializers,
    InitializeApplicationConfigsResponse object, {
    FullType specifiedType = FullType.unspecified,
  }) {
    return _serializeProperties(serializers, object, specifiedType: specifiedType).toList();
  }

  void _deserializeProperties(
    Serializers serializers,
    Object serialized, {
    FullType specifiedType = FullType.unspecified,
    required List<Object?> serializedList,
    required InitializeApplicationConfigsResponseBuilder result,
    required List<Object?> unhandled,
  }) {
    for (var i = 0; i < serializedList.length; i += 2) {
      final key = serializedList[i] as String;
      final value = serializedList[i + 1];
      switch (key) {
        case r'binding_id':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(String),
          ) as String;
          result.bindingId = valueDes;
          break;
        case r'created':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(bool),
          ) as bool;
          result.created = valueDes;
          break;
        case r'file_count':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(int),
          ) as int;
          result.fileCount = valueDes;
          break;
        case r'status':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(String),
          ) as String;
          result.status = valueDes;
          break;
        default:
          unhandled.add(key);
          unhandled.add(value);
          break;
      }
    }
  }

  @override
  InitializeApplicationConfigsResponse deserialize(
    Serializers serializers,
    Object serialized, {
    FullType specifiedType = FullType.unspecified,
  }) {
    final result = InitializeApplicationConfigsResponseBuilder();
    final serializedList = (serialized as Iterable<Object?>).toList();
    final unhandled = <Object?>[];
    _deserializeProperties(
      serializers,
      serialized,
      specifiedType: specifiedType,
      serializedList: serializedList,
      unhandled: unhandled,
      result: result,
    );
    return result.build();
  }
}
